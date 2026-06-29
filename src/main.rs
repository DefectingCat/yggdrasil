//! 服务端入口与启动配置
//!
//! 本文件是 Dioxus fullstack 应用的启动入口。
//! 当启用 `server` feature 时，启动 Axum 服务器并挂载：
//! - Dioxus server function（由 `serve_dioxus_application` 自动注册）；
//! - 自定义 Axum 路由：图片上传 `/api/upload`、图片服务 `/uploads/{*path}`；
//! - 增量渲染（Incremental Rendering）缓存配置。
//!
//! 当未启用 `server` feature（例如编译为 WASM 前端）时，
//! 仅调用 `dioxus::launch` 启动客户端应用。

// 业务模块
mod api;
mod auth;
mod cache;
mod components;
mod context;
mod db;
// highlight 模块仅在服务端构建时编译
#[cfg(feature = "server")]
mod highlight;
mod hooks;
mod models;
mod pages;
mod router;
// ssr_cache 仅在 server feature 启用时编译；保存 SSR 世代号失效状态。
#[cfg(feature = "server")]
mod ssr_cache;
mod tasks;
mod theme;
// tiptap_bridge：共享类型（UploadsInFlight/UploadErrorEntry）两端都编译；
// wasm-bindgen extern 与 EditorHandle 在内部的 #[cfg(wasm32)] 子模块里。
mod tiptap_bridge;
mod utils;
mod webp;

/// 压缩算法配置。
#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompressionAlgorithms {
    gzip: bool,
    brotli: bool,
    deflate: bool,
    zstd: bool,
}

#[cfg(feature = "server")]
impl CompressionAlgorithms {
    fn all_enabled() -> Self {
        Self {
            gzip: true,
            brotli: true,
            deflate: true,
            zstd: true,
        }
    }

    fn is_empty(&self) -> bool {
        !self.gzip && !self.brotli && !self.deflate && !self.zstd
    }
}

/// 解析 COMPRESSION_ALGORITHMS 环境变量值。
/// ""、"none"、"off" 返回 None；"all" 或未识别到任何算法时启用全部。
#[cfg(feature = "server")]
fn parse_compression_algorithms(env: &str) -> Option<CompressionAlgorithms> {
    let env = env.trim();
    if env.is_empty() || env.eq_ignore_ascii_case("none") || env.eq_ignore_ascii_case("off") {
        return None;
    }

    let mut all = false;
    let mut gzip = false;
    let mut brotli = false;
    let mut deflate = false;
    let mut zstd = false;

    for part in env.split(',') {
        match part.trim().to_lowercase().as_str() {
            "all" => all = true,
            "gzip" => gzip = true,
            "brotli" | "br" => brotli = true,
            "deflate" => deflate = true,
            "zstd" => zstd = true,
            other => tracing::warn!(
                "Unknown compression algorithm in COMPRESSION_ALGORITHMS: '{}'",
                other
            ),
        }
    }

    if all {
        return Some(CompressionAlgorithms::all_enabled());
    }

    let algorithms = CompressionAlgorithms {
        gzip,
        brotli,
        deflate,
        zstd,
    };
    if algorithms.is_empty() {
        return None;
    }

    Some(algorithms)
}

/// 根据 COMPRESSION_ALGORITHMS 环境变量构造 CompressionLayer。
/// 未设置或设置为 "all" 时启用全部算法；设置为 ""、"none" 或 "off" 时禁用。
#[cfg(feature = "server")]
fn compression_layer_from_env() -> Option<tower_http::compression::CompressionLayer> {
    use tower_http::compression::CompressionLayer;

    let env = std::env::var("COMPRESSION_ALGORITHMS").unwrap_or_else(|_| "all".to_string());
    let algorithms = parse_compression_algorithms(&env)?;

    Some(
        CompressionLayer::new()
            .gzip(algorithms.gzip)
            .br(algorithms.brotli)
            .deflate(algorithms.deflate)
            .zstd(algorithms.zstd),
    )
}

/// 根据请求路径和方法决定公开页面的 Cache-Control 头。
/// 返回 None 表示不添加缓存头（保留现有行为或避免覆盖）。
#[cfg(feature = "server")]
fn cache_control_for_path(
    path: &str,
    method: &axum::http::Method,
) -> Option<axum::http::HeaderValue> {
    use axum::http::{HeaderValue, Method};

    // 只对 GET/HEAD 请求添加缓存头
    if *method != Method::GET && *method != Method::HEAD {
        return None;
    }

    // API 接口：不缓存（可能涉及认证、写操作）
    if path.starts_with("/api") {
        return None;
    }

    // 管理后台和认证页面：不缓存
    if path.starts_with("/admin") || path == "/login" || path == "/register" {
        return None;
    }

    // 静态资源：长期缓存（Dioxus/WASM 资源通常带内容哈希）
    if path.starts_with("/_dioxus/")
        || path.starts_with("/wasm/")
        || path.ends_with(".wasm")
        || path.ends_with(".js")
        || path == "/style.css"
        || path == "/highlight.css"
    {
        return Some(HeaderValue::from_static("public, max-age=31536000, immutable"));
    }

    // 公开页面：5 分钟新鲜期，过期后 1 小时内可提供过期内容并后台重新验证
    Some(HeaderValue::from_static(
        "public, max-age=300, stale-while-revalidate=3600",
    ))
}

/// Axum 中间件：为公开页面和静态资源附加 Cache-Control 头。
#[cfg(feature = "server")]
async fn add_cache_control(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::header;

    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let cache_value = cache_control_for_path(&path, &method);

    let mut response = next.run(req).await;

    if let Some(value) = cache_value {
        // 仅当响应尚未设置 Cache-Control 时才添加，避免覆盖已有策略
        response.headers_mut().entry(header::CACHE_CONTROL).or_insert(value);
    }

    response
}

/// 程序入口
fn main() {
    // server feature：启动服务端
    #[cfg(feature = "server")]
    {
        // 加载 .env 环境变量
        dotenvy::dotenv().ok();
        // 初始化 tracing 日志，默认级别为 info
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();

        // 校验数据库连接串，未设置则直接退出
        if std::env::var("DATABASE_URL").is_err() {
            tracing::error!("DATABASE_URL environment variable not set. Make sure .env exists or the variable is exported.");
            eprintln!("ERROR: DATABASE_URL environment variable not set");
            eprintln!("HINT: create a .env file with DATABASE_URL=postgres://user:pass@host:5432/dbname");
            std::process::exit(1);
        }

        // 前置校验 DATABASE_URL 格式 + DB_POOL_SIZE，避免触发 DB_POOL LazyLock 闭包里
        // 不可达的 .expect() panic——让用户可修复的配置错误走统一友好的 exit(1) 路径。
        // 此处必须在任何 DB_POOL.get() 调用之前执行（即迁移之前）。
        if let Err(e) = db::pool::validate_database_url() {
            tracing::error!("{e}");
            eprintln!("ERROR: {e}");
            if e.starts_with("DB_POOL_SIZE") {
                eprintln!("HINT: DB_POOL_SIZE must be a positive integer (e.g. 20).");
            } else {
                eprintln!("HINT: expected something like postgres://user:pass@host:5432/dbname");
            }
            std::process::exit(1);
        }

        // 提醒部署者显式设置 APP_BASE_URL：未设置时 CSRF 会回退到 Host 头，
        // 反向代理后存在绕过风险（本地 localhost 开发会静默，不打日志）。
        api::csrf::warn_if_app_base_url_unset();

        // 启动前执行数据库迁移。阻塞：完成前不监听端口。
        // 失败用 exit(1) 退出（不 panic），避免启动一个 schema 不一致的半残服务。
        // 多实例滚动发布时由咨询锁串行化，详见 src/db/migrate.rs。
        //
        // main() 是同步函数，这里用一个独立的多线程 runtime 驱动迁移的异步逻辑，
        // 完成后再交给 dioxus::server::serve() 启动它自己的 runtime。
        // 两个 runtime 不重叠，避免与 Dioxus 内部 runtime 产生交互。
        let migrate_rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build migration runtime");
        migrate_rt.block_on(async {
            tracing::info!("running database migrations");

            // 连接池指向目标库，但目标库可能尚不存在（全新部署）。
            // 先连 postgres 维护库确保目标库存在，复用启动超时窗口应对 DB 起得慢。
            if let Err(e) = db::pool::ensure_database().await {
                tracing::error!("failed to ensure target database exists: {e}");
                eprintln!("ERROR: failed to ensure target database exists: {e}");
                eprintln!("HINT: verify DATABASE_URL; the role needs CREATEDB (or CREATE privilege on the 'postgres' DB) to auto-create the target database.");
                std::process::exit(1);
            }

            // 启动期用长重试窗口拿连接：DB 可能还在初始化（docker-compose 无 healthcheck、
            // 本机忘启 Postgres 等）。窗口由 MIGRATE_STARTUP_TIMEOUT_SECS 控制，默认 30s。
            let mut conn = match db::pool::get_conn_for_startup().await {
                Ok(conn) => conn,
                Err(e) => {
                    let secs = std::env::var("MIGRATE_STARTUP_TIMEOUT_SECS")
                        .ok()
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(30);
                    tracing::error!("could not connect to database within {secs}s startup window: {e}");
                    eprintln!("ERROR: could not connect to database within {secs}s startup window: {e}");
                    eprintln!("HINT: is PostgreSQL running and reachable at the configured DATABASE_URL?");
                    eprintln!("HINT: raise MIGRATE_STARTUP_TIMEOUT_SECS if the DB needs longer to start.");
                    std::process::exit(1);
                }
            };

            // 连接拿到后再执行迁移主体（咨询锁 + 建表 + 应用迁移）。
            if let Err(e) = db::migrate::run_on_conn(&mut conn).await {
                tracing::error!("database migration failed: {e}");
                eprintln!("ERROR: database migration failed: {e}");
                eprintln!("HINT: check the logs above; verify DATABASE_URL and that PostgreSQL is healthy.");
                std::process::exit(1);
            }
        });
        // 迁移 runtime 用完即弃，显式 drop 以在 serve() 前释放其线程资源。
        drop(migrate_rt);

        // 启动 Dioxus 服务端，返回构建好的 Axum Router
        dioxus::server::serve(|| async move {
            use dioxus::server::{axum, DioxusRouterExt, ServeConfig};
            use std::time::Duration;
            use tower_http::timeout::TimeoutLayer;
            use axum::http::StatusCode;

            // 启动后台定时任务：IP 信息清理
            tokio::spawn(async {
                tasks::ip_purge::run_purge().await;
            });

            // 启动后台定时任务：过期 session 清理
            tokio::spawn(async {
                tasks::session_cleanup::run_cleanup().await;
            });

            // 启动后台定时任务：回收站自动清理
            tokio::spawn(async {
                tasks::post_purge::run_purge().await;
            });

            // 启动后台定时任务：图片磁盘缓存清理
            tokio::spawn(async {
                tasks::image_cache_cleanup::run_cleanup().await;
            });

            // 配置增量渲染缓存，默认缓存 3600 秒，可通过 SSR_CACHE_SECS 覆盖。
            // 注意：src/ssr_cache.rs 中的世代号是未来就绪基础设施，当前并不会使
            // Dioxus 0.7 的 SSR 缓存实际失效（Dioxus 未暴露相应 API）。在 API 可用
            // 之前，SSR_CACHE_SECS 仍是唯一有效的兜底 TTL。
            let config = ServeConfig::builder().incremental(
                dioxus::server::IncrementalRendererConfig::default().invalidate_after(
                    std::time::Duration::from_secs(
                        std::env::var("SSR_CACHE_SECS")
                            .ok()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(3600),
                    ),
                ),
            );

            // SSR 世代号中间件：把当前全局世代号注入请求扩展，并对 GET 请求的
            // 响应附加 `X-SSR-Generation` 头。这是为未来 Dioxus 支持自定义 SSR 缓存键
            // 预留的钩子；目前主要提供可观测性，不会实际失效 SSR 缓存。
            async fn ssr_generation_middleware(
                req: axum::http::Request<axum::body::Body>,
                next: axum::middleware::Next,
            ) -> axum::response::Response {
                let generation = crate::ssr_cache::current_global_generation();
                let is_get = req.method() == axum::http::Method::GET;
                let (mut parts, body) = req.into_parts();
                parts.extensions.insert(crate::ssr_cache::SsrGeneration(generation));
                let mut response = next.run(axum::http::Request::from_parts(parts, body)).await;
                if is_get {
                    response.headers_mut().insert(
                        axum::http::header::HeaderName::from_static("x-ssr-generation"),
                        axum::http::HeaderValue::from_str(&generation.to_string())
                            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
                    );
                }
                response
            }

            // 自定义 API 路由：图片上传（大文件，需要更长超时）
            // CSRF 校验置于最外层，先拦截非法来源再做超时/限体。
            let upload_route = axum::Router::new()
                .route(
                    "/api/upload",
                    axum::routing::post(crate::api::upload::upload_image),
                )
                .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_secs(300),
                ))
                .layer(axum::middleware::from_fn(
                    crate::api::csrf::csrf_middleware,
                ));

            // Dioxus 应用路由：自动挂载所有 server function 并渲染前端组件
            let dioxus_app =
                axum::Router::new().serve_dioxus_application(config, router::AppRouter);

            // 合并 Dioxus + CSRF/世代号/缓存头/可选压缩/30s 超时中间件
            // layer 顺序：后加的最外层先执行。CSRF 最外层先拦截非法来源。
            let mut app_routes = dioxus_app
                .layer(axum::middleware::from_fn(ssr_generation_middleware))
                .layer(axum::middleware::from_fn(add_cache_control))
                .layer(axum::middleware::from_fn(
                    crate::api::csrf::csrf_middleware,
                ));
            if let Some(layer) = compression_layer_from_env() {
                app_routes = app_routes.layer(layer);
            }
            let app_routes = app_routes.layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_secs(30),
            ));

            // 静态资源路由：图片文件服务。
            // 注意：`dioxus::server::serve()` 接管了 listener 与 `into_make_service`
            // 调用，没有机会换成 `into_make_service_with_connect_info::<SocketAddr>()`，
            // 所以手动 merge 进来的路由（含 static_routes）拿不到 `ConnectInfo` 扩展。
            // serve_image / upload_image 因此都用 `Option<Extension<ConnectInfo<SocketAddr>>>`
            // 优雅降级。生产环境应在反向代理后部署并配置 TRUSTED_PROXY_COUNT，
            // 使限流能拿到真实客户端 IP。
            let static_routes = axum::Router::new()
                .route(
                    "/healthz",
                    axum::routing::get(crate::api::health::healthz),
                )
                .route(
                    "/readyz",
                    axum::routing::get(crate::api::health::readyz),
                )
                .route(
                    "/uploads/{*path}",
                    axum::routing::get(crate::api::image::serve_image),
                )
                .route(
                    "/uploads",
                    axum::routing::get(|| async { StatusCode::NOT_FOUND }),
                );

            // 合并：upload 路由保持自己独立的 300s 超时；app routes 加可选压缩/30s；static routes 无任何中间件
            let router = upload_route.merge(app_routes).merge(static_routes);

            Ok(router)
        });
    }

    // 非 server feature（通常为 WASM 前端）：启动客户端应用
    #[cfg(not(feature = "server"))]
    {
        use router::AppRouter;
        dioxus::launch(AppRouter);
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{cache_control_for_path, parse_compression_algorithms, CompressionAlgorithms};
    use axum::http::Method;

    fn cache_value(path: &str, method: Method) -> Option<String> {
        cache_control_for_path(path, &method)
            .map(|v| v.to_str().unwrap().to_string())
    }

    #[test]
    fn public_page_is_cached() {
        assert_eq!(
            cache_value("/", Method::GET),
            Some("public, max-age=300, stale-while-revalidate=3600".to_string())
        );
        assert_eq!(
            cache_value("/post/hello-world", Method::GET),
            Some("public, max-age=300, stale-while-revalidate=3600".to_string())
        );
        assert_eq!(
            cache_value("/tags/rust", Method::GET),
            Some("public, max-age=300, stale-while-revalidate=3600".to_string())
        );
    }

    #[test]
    fn static_assets_are_cached_long_term() {
        assert_eq!(
            cache_value("/style.css", Method::GET),
            Some("public, max-age=31536000, immutable".to_string())
        );
        assert_eq!(
            cache_value("/highlight.css", Method::GET),
            Some("public, max-age=31536000, immutable".to_string())
        );
        assert_eq!(
            cache_value("/wasm/app.wasm", Method::GET),
            Some("public, max-age=31536000, immutable".to_string())
        );
        assert_eq!(
            cache_value("/_dioxus/assets/main.js", Method::GET),
            Some("public, max-age=31536000, immutable".to_string())
        );
    }

    #[test]
    fn api_and_admin_and_auth_are_not_cached() {
        assert_eq!(cache_value("/api/posts", Method::GET), None);
        assert_eq!(cache_value("/admin", Method::GET), None);
        assert_eq!(cache_value("/admin/posts", Method::GET), None);
        assert_eq!(cache_value("/login", Method::GET), None);
        assert_eq!(cache_value("/register", Method::GET), None);
    }

    #[test]
    fn non_get_requests_are_not_cached() {
        assert_eq!(cache_value("/", Method::POST), None);
        assert_eq!(cache_value("/post/hello-world", Method::POST), None);
        assert_eq!(cache_value("/style.css", Method::POST), None);
    }

    #[test]
    fn head_requests_are_cached_like_get() {
        assert_eq!(
            cache_value("/", Method::HEAD),
            Some("public, max-age=300, stale-while-revalidate=3600".to_string())
        );
    }

    #[test]
    fn compression_all_enables_everything() {
        assert_eq!(
            parse_compression_algorithms("all"),
            Some(CompressionAlgorithms::all_enabled())
        );
    }

    #[test]
    fn compression_default_env_is_all() {
        // 模拟未设置环境变量时的默认值
        assert_eq!(
            parse_compression_algorithms("all"),
            Some(CompressionAlgorithms::all_enabled())
        );
    }

    #[test]
    fn compression_empty_none_off_disable() {
        assert_eq!(parse_compression_algorithms(""), None);
        assert_eq!(parse_compression_algorithms("none"), None);
        assert_eq!(parse_compression_algorithms("NONE"), None);
        assert_eq!(parse_compression_algorithms("off"), None);
        assert_eq!(parse_compression_algorithms("OFF"), None);
    }

    #[test]
    fn compression_single_algorithm() {
        assert_eq!(
            parse_compression_algorithms("gzip"),
            Some(CompressionAlgorithms {
                gzip: true,
                brotli: false,
                deflate: false,
                zstd: false,
            })
        );
        assert_eq!(
            parse_compression_algorithms("br"),
            Some(CompressionAlgorithms {
                gzip: false,
                brotli: true,
                deflate: false,
                zstd: false,
            })
        );
    }

    #[test]
    fn compression_multiple_algorithms() {
        assert_eq!(
            parse_compression_algorithms("gzip, zstd"),
            Some(CompressionAlgorithms {
                gzip: true,
                brotli: false,
                deflate: false,
                zstd: true,
            })
        );
    }

    #[test]
    fn compression_case_insensitive_and_whitespace_tolerant() {
        assert_eq!(
            parse_compression_algorithms("GZIP, Brotli, Deflate, Zstd"),
            Some(CompressionAlgorithms::all_enabled())
        );
        assert_eq!(
            parse_compression_algorithms(" gzip , br , deflate , zstd "),
            Some(CompressionAlgorithms::all_enabled())
        );
    }

    #[test]
    fn compression_unknown_algorithms_are_ignored() {
        assert_eq!(
            parse_compression_algorithms("gzip, unknown, lz4"),
            Some(CompressionAlgorithms {
                gzip: true,
                brotli: false,
                deflate: false,
                zstd: false,
            })
        );
    }
}

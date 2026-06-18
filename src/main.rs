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
mod utils;
mod webp;

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
            std::process::exit(1);
        }

        // 启动 Dioxus 服务端，返回构建好的 Axum Router
        dioxus::server::serve(|| async move {
            use dioxus::server::{axum, DioxusRouterExt, ServeConfig};
            use std::time::Duration;
            use tower_http::compression::CompressionLayer;
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
            // 注意：世代号失效机制已就位（见 src/ssr_cache.rs），但 Dioxus 0.7 未暴露
            // 自定义缓存键 API，因此 TTL 仍是当前有效的兜底策略。
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

            // SSR 世代号中间件：把当前全局世代号注入请求扩展并附加到响应头。
            // 这是为 Dioxus 未来支持自定义 SSR 缓存键预留的钩子；目前主要提供可观测性。
            async fn ssr_generation_middleware(
                req: axum::http::Request<axum::body::Body>,
                next: axum::middleware::Next,
            ) -> axum::response::Response {
                let generation = crate::ssr_cache::current_global_generation();
                let (mut parts, body) = req.into_parts();
                parts.extensions.insert(crate::ssr_cache::SsrGeneration(generation));
                let mut response = next.run(axum::http::Request::from_parts(parts, body)).await;
                response.headers_mut().insert(
                    axum::http::header::HeaderName::from_static("x-ssr-generation"),
                    axum::http::HeaderValue::from_str(&generation.to_string())
                        .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
                );
                response
            }

            // 自定义 API 路由：图片上传（大文件，需要更长超时）
            let upload_route = axum::Router::new()
                .route(
                    "/api/upload",
                    axum::routing::post(crate::api::upload::upload_image),
                )
                .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_secs(300),
                ));

            // Dioxus 应用路由：自动挂载所有 server function 并渲染前端组件
            let dioxus_app =
                axum::Router::new().serve_dioxus_application(config, router::AppRouter);

            // 合并 Dioxus + 世代号/压缩/30s 超时中间件
            let app_routes = dioxus_app
                .layer(axum::middleware::from_fn(ssr_generation_middleware))
                .layer(CompressionLayer::new())
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_secs(30),
                ));

            // 静态资源路由：图片文件服务。
            // 注意：axum 0.8 没有 ConnectInfoLayer，且 dioxus::server::serve 不会把
            // ConnectInfo 扩展传播到手动 merge 的路由，所以 serve_image 使用
            // Option<Extension<ConnectInfo<SocketAddr>>> 优雅降级。生产环境应在反向代理后
            // 部署并配置 TRUSTED_PROXY_COUNT，使限流能拿到真实客户端 IP。
            let static_routes = axum::Router::new()
                .route(
                    "/uploads/{*path}",
                    axum::routing::get(crate::api::image::serve_image),
                )
                .route(
                    "/uploads",
                    axum::routing::get(|| async { StatusCode::NOT_FOUND }),
                );

            // 合并：upload 路由保持自己独立的 300s 超时；app routes 加压缩/30s；static routes 无任何中间件
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

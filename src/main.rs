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
#[cfg(feature = "server")]
mod models;
mod pages;
mod router;
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

            // 配置增量渲染缓存，默认缓存 3600 秒，可通过 SSR_CACHE_SECS 覆盖
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

            // 自定义 API 路由：图片上传，设置最大请求体大小为 10 MiB
            // （包含 multipart 开销，实际文件限制由 upload_image 内 MAX_FILE_SIZE 控制）
            let api_routes = axum::Router::new().route(
                "/api/upload",
                axum::routing::post(crate::api::upload::upload_image)
                    .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024)),
            );

            // 静态资源路由：图片文件服务，支持动态裁剪/旋转/格式转换
            let static_routes = axum::Router::new().route(
                "/uploads/{*path}",
                axum::routing::get(crate::api::image::serve_image),
            );

            // Dioxus 应用路由：自动挂载所有 server function 并渲染前端组件
            let dioxus_app =
                axum::Router::new().serve_dioxus_application(config, router::AppRouter);

            // 合并三条路由：自定义 API、静态资源、Dioxus 主应用
            let router = api_routes.merge(static_routes).merge(dioxus_app);

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

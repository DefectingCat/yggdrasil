mod api;
mod cache;
mod auth;
mod components;
mod context;
mod db;
#[cfg(feature = "server")]
mod highlight;
mod hooks;
mod models;
mod pages;
mod router;
mod tasks;
mod theme;
mod utils;
mod webp;

fn main() {
    #[cfg(feature = "server")]
    {
        dotenvy::dotenv().ok();
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();

        if std::env::var("DATABASE_URL").is_err() {
            tracing::error!("DATABASE_URL environment variable not set. Make sure .env exists or the variable is exported.");
            eprintln!("ERROR: DATABASE_URL environment variable not set");
            std::process::exit(1);
        }

        dioxus::server::serve(|| async move {
            use dioxus::server::{axum, DioxusRouterExt, ServeConfig};

            tokio::spawn(async {
                tasks::ip_purge::run_purge().await;
            });

            tokio::spawn(async {
                tasks::session_cleanup::run_cleanup().await;
            });

            let config = ServeConfig::builder().incremental(
                dioxus::server::IncrementalRendererConfig::default()
                    .invalidate_after(std::time::Duration::from_secs(
                        std::env::var("SSR_CACHE_SECS")
                            .ok()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(3600),
                    )),
            );
            let api_routes = axum::Router::new().route(
                "/api/upload",
                axum::routing::post(crate::api::upload::upload_image)
                    .layer(axum::extract::DefaultBodyLimit::disable()),
            );

            let static_routes = axum::Router::new().route(
                "/uploads/{*path}",
                axum::routing::get(crate::api::image::serve_image),
            );

            let dioxus_app =
                axum::Router::new().serve_dioxus_application(config, router::AppRouter);

            let router = api_routes
                .merge(static_routes)
                .merge(dioxus_app);

            Ok(router)
        });
    }

    #[cfg(not(feature = "server"))]
    {
        use router::AppRouter;
        dioxus::launch(AppRouter);
    }
}

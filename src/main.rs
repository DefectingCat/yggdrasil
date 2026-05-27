mod api;
mod auth;
mod components;
mod context;
mod db;
mod models;
mod pages;
mod router;
mod tasks;
mod theme;

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

        dioxus::server::serve(|| async move {
            use dioxus::server::{axum, DioxusRouterExt, ServeConfig};
            use tower_http::trace::TraceLayer;

            tokio::spawn(async {
                tasks::session_cleanup::run_cleanup().await;
            });

            let config = ServeConfig::new();
            let router = axum::Router::new()
                .layer(TraceLayer::new_for_http())
                .serve_dioxus_application(config, router::AppRouter);

            Ok(router)
        });
    }

    #[cfg(not(feature = "server"))]
    {
        use router::AppRouter;
        dioxus::launch(AppRouter);
    }
}

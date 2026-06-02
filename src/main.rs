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

        if std::env::var("DATABASE_URL").is_err() {
            tracing::error!("DATABASE_URL environment variable not set. Make sure .env exists or the variable is exported.");
            eprintln!("ERROR: DATABASE_URL environment variable not set");
            std::process::exit(1);
        }

        dioxus::server::serve(|| async move {
            use dioxus::server::{axum, DioxusRouterExt, ServeConfig};
            use tower_http::trace::TraceLayer;
            use tracing::Level;

            tokio::spawn(async {
                tasks::session_cleanup::run_cleanup().await;
            });

            let config = ServeConfig::new();
            let router = axum::Router::new()
                .serve_dioxus_application(config, router::AppRouter)
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(
                            tower_http::trace::DefaultMakeSpan::new().level(Level::INFO),
                        )
                        .on_request(tower_http::trace::DefaultOnRequest::new().level(Level::INFO))
                        .on_response(
                            tower_http::trace::DefaultOnResponse::new().level(Level::INFO),
                        ),
                );

            Ok(router)
        });
    }

    #[cfg(not(feature = "server"))]
    {
        use router::AppRouter;
        dioxus::launch(AppRouter);
    }
}

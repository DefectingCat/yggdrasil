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

use router::AppRouter;

fn main() {
    #[cfg(feature = "server")]
    {
        dotenvy::dotenv().ok();
        std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(tasks::session_cleanup::run_cleanup());
        });
    }

    dioxus::launch(AppRouter);
}

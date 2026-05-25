use dioxus::prelude::*;

mod api;
mod auth;
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
        tokio::spawn(tasks::session_cleanup::run_cleanup());
    }

    dioxus::launch(AppRouter);
}

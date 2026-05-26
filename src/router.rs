use dioxus::prelude::*;

use crate::pages::admin::AdminPage;
use crate::pages::home::HomePage;
use crate::pages::login::LoginPage;
use crate::pages::register::RegisterPage;
use crate::theme::{Theme, ThemePreload, use_theme_provider};

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    HomePage {},
    #[route("/login")]
    LoginPage {},
    #[route("/register")]
    RegisterPage {},
    #[route("/admin")]
    AdminPage {},
    #[route("/archives")]
    ArchivesPage {},
    #[route("/tags")]
    TagsPage {},
    #[route("/search")]
    SearchPage {},
    #[route("/about")]
    AboutPage {},
}

#[component]
pub fn AppRouter() -> Element {
    let theme = use_theme_provider();
    let theme_class = match theme() {
        Theme::Dark => "dark",
        Theme::Light => "",
    };

    rsx! {
        div {
            class: "{theme_class}",
            ThemePreload {}
            Router::<Route> {}
        }
    }
}

#[component]
pub fn ArchivesPage() -> Element {
    rsx! { "Archives" }
}

#[component]
pub fn TagsPage() -> Element {
    rsx! { "Tags" }
}

#[component]
pub fn SearchPage() -> Element {
    rsx! { "Search" }
}

#[component]
pub fn AboutPage() -> Element {
    rsx! { "About" }
}

use dioxus::prelude::*;
use std::sync::Arc;

use crate::components::admin_layout::AdminLayout;
use crate::context::UserContext;
use crate::pages::admin::{AdminPage, WritePage};
use crate::pages::archives::ArchivesPage;
use crate::pages::home::HomePage;
use crate::pages::login::LoginPage;
use crate::pages::register::RegisterPage;
use crate::pages::tags::{TagsPage, TagDetailPage};
use crate::theme::{Theme, ThemePreload, use_theme_provider};

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/")]
    HomePage {},
    #[route("/login")]
    LoginPage {},
    #[route("/register")]
    RegisterPage {},

    #[nest("/admin")]
    #[layout(AdminLayout)]
        #[route("/")]
        AdminPage {},
        #[route("/write")]
        WritePage {},
    #[end_layout]
    #[end_nest]

    #[route("/archives")]
    ArchivesPage {},
    #[route("/tags")]
    TagsPage {},
    #[route("/tags/:tag")]
    TagDetailPage { tag: String },
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

    let user = use_signal(|| None::<Arc<crate::models::user::User>>);
    let checked = use_signal(|| false);
    use_context_provider(|| UserContext { user, checked });

    rsx! {
        div {
            class: "{theme_class}",
            ThemePreload {}
            Router::<Route> {}
        }
    }
}

#[component]
pub fn SearchPage() -> Element {
    rsx! { "Search" }
}

#[component]
pub fn AboutPage() -> Element {
    rsx! { "About" }
}

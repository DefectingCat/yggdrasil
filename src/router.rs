use dioxus::prelude::*;
use std::sync::Arc;

use crate::components::admin_layout::AdminLayout;
use crate::components::frontend_layout::FrontendLayout;
use crate::context::UserContext;
use crate::pages::about::About;
use crate::pages::admin::{Admin, Posts, Write};
use crate::pages::archives::Archives;
use crate::pages::home::{Home, HomePage};
use crate::pages::login::Login;
use crate::pages::post_detail::PostDetail;
use crate::pages::register::Register;
use crate::pages::search::Search;
use crate::pages::tags::{TagDetail, Tags};
use crate::theme::{use_theme_provider, Theme, ThemePreload};

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(FrontendLayout)]
        #[route("/")]
        Home {},
        #[route("/page/:page")]
        HomePage { page: i32 },
        #[route("/archives", wasm_split)]
        Archives {},
        #[route("/tags", wasm_split)]
        Tags {},
        #[route("/tags/:tag", wasm_split)]
        TagDetail { tag: String },
        #[route("/post/:slug")]
        PostDetail { slug: String },
        #[route("/search", wasm_split)]
        Search {},
        #[route("/about", wasm_split)]
        About {},
    #[end_layout]

    #[nest("/admin")]
    #[layout(AdminLayout)]
        #[route("/", wasm_split)]
        Admin {},
        #[route("/write", wasm_split)]
        Write {},
        #[route("/posts", wasm_split)]
        Posts {},
    #[end_layout]
    #[end_nest]

    #[route("/login", wasm_split)]
    Login {},
    #[route("/register", wasm_split)]
    Register {},
}

#[component]
pub fn AppRouter() -> Element {
    let theme = use_theme_provider();
    let theme_class = match theme() {
        Theme::Dark => "dark",
        Theme::Light => "",
    };

    let user = use_signal(|| None::<Arc<crate::models::user::PublicUser>>);
    let checked = use_signal(|| false);
    use_context_provider(|| UserContext { user, checked });

    rsx! {
        document::Stylesheet { href: "/style.css" }
        document::Stylesheet { href: "/highlight.css" }
        document::Title { "Yggdrasil Blog" }
        div {
            class: "{theme_class}",
            ThemePreload {}
            Router::<Route> {}
        }
    }
}

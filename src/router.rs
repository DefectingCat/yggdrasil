use dioxus::prelude::*;
use std::sync::Arc;

use crate::components::admin_layout::AdminLayout;
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
    #[route("/")]
    Home {},
    #[route("/page/:page")]
    HomePage { page: i32 },
    #[route("/login")]
    Login {},
    #[route("/register")]
    Register {},

    #[nest("/admin")]
    #[layout(AdminLayout)]
        #[route("/")]
        Admin {},
        #[route("/write")]
        Write {},
        #[route("/posts")]
        Posts {},
    #[end_layout]
    #[end_nest]

    #[route("/archives")]
    Archives {},
    #[route("/tags")]
    Tags {},
    #[route("/tags/:tag")]
    TagDetail { tag: String },
    #[route("/post/:slug")]
    PostDetail { slug: String },
    #[route("/search")]
    Search {},
    #[route("/about")]
    About {},
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
        div {
            class: "{theme_class}",
            ThemePreload {}
            Router::<Route> {}
        }
    }
}

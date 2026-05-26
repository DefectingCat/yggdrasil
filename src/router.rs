use dioxus::prelude::*;

use crate::pages::admin::AdminPage;
use crate::pages::login::LoginPage;
use crate::pages::register::RegisterPage;
use crate::theme::{Theme, ThemeToggle, use_theme};

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    #[route("/login")]
    LoginPage {},
    #[route("/register")]
    RegisterPage {},
    #[route("/admin")]
    AdminPage {},
}

#[component]
pub fn AppRouter() -> Element {
    let theme = use_theme();
    let theme_class = match theme() {
        Theme::Dark => "dark",
        Theme::Light => "",
    };

    rsx! {
        div {
            class: theme_class,
            ThemeToggle {}
            Router::<Route> {}
        }
    }
}


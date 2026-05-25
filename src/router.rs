use dioxus::prelude::*;

use crate::pages::admin::AdminPage;
use crate::pages::login::LoginPage;
use crate::pages::register::RegisterPage;
use crate::theme::{Theme, ThemeToggle, use_theme};

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
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

#[component]
pub fn Home() -> Element {
    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900",
            div { class: "text-center",
                h1 { class: "text-4xl font-bold text-gray-900 dark:text-white mb-4",
                    "Yggdrasil Blog"
                }
                p { class: "text-gray-600 dark:text-gray-300 mb-8",
                    "以文字为主的简约博客系统"
                }
                div { class: "space-x-4",
                    a {
                        class: "px-6 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors",
                        href: "/login",
                        "登录"
                    }
                    a {
                        class: "px-6 py-2 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-900 dark:text-white rounded-lg transition-colors",
                        href: "/register",
                        "注册"
                    }
                }
            }
        }
    }
}

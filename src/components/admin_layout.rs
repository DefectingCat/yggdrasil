use dioxus::prelude::*;

use crate::api::auth::{get_current_user, logout};
use crate::components::header::{Header, NavItemConfig};
use crate::components::footer::Footer;
use crate::router::Route;

#[component]
pub fn AdminLayout(children: Element) -> Element {
    let user_resource =
        use_resource(|| async move { get_current_user().await.ok().and_then(|r| r.user) });

    let navigator = dioxus::router::navigator();
    let route = use_route::<Route>();

    let admin_nav_items = vec![
        NavItemConfig {
            href: "/admin",
            label: "仪表盘",
            is_active: matches!(route, Route::AdminPage {}),
        },
        NavItemConfig {
            href: "/admin/write",
            label: "写文章",
            is_active: matches!(route, Route::WritePage {}),
        },
        NavItemConfig {
            href: "/",
            label: "前台",
            is_active: false,
        },
    ];

    let nav = navigator;
    let logout_button = rsx! {
        button {
            class: "text-sm text-gray-600 dark:text-[#9b9c9d] hover:text-gray-900 dark:hover:text-[#dadadb] transition-colors",
            onclick: move |_| {
                let nav = nav;
                spawn(async move {
                    let _ = logout().await;
                    let _ = nav.push("/login");
                });
            },
            "登出"
        }
    };

    let user_data = user_resource.read().clone();

    let should_redirect = matches!(user_data.as_ref(), Some(None));

    use_effect(move || {
        if should_redirect {
            navigator.push("/login");
        }
    });

    match user_data.as_ref() {
        Some(Some(_user)) => {
            rsx! {
                div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20]",
                    Header { nav_items: admin_nav_items, right_content: logout_button }
                    main { class: "flex-1 w-full max-w-5xl mx-auto px-6 py-8",
                        {children}
                    }
                    Footer {}
                }
            }
        }
        _ => {
            rsx! {
                div { class: "min-h-screen flex items-center justify-center bg-white dark:bg-[#1d1e20]",
                    p { class: "text-gray-600 dark:text-[#9b9c9d]", "加载中..." }
                }
            }
        }
    }
}

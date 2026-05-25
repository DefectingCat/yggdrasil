use dioxus::prelude::*;

use crate::api::auth::{get_current_user, logout};

#[component]
pub fn AdminPage() -> Element {
    let user_resource = use_resource(|| async move {
        get_current_user().await.ok().and_then(|r| r.user)
    });

    let navigator = dioxus::router::navigator();

    let user_data = user_resource.read().clone();

    match user_data.as_ref() {
        Some(Some(user)) => {
            let username = user.username.clone();
            rsx! {
                div { class: "min-h-screen bg-gray-50 dark:bg-gray-900",
                    header { class: "bg-white dark:bg-gray-800 shadow",
                        div { class: "max-w-7xl mx-auto px-4 py-4 flex justify-between items-center",
                            h1 { class: "text-xl font-bold text-gray-900 dark:text-white",
                                "后台管理"
                            }
                            div { class: "flex items-center gap-4",
                                span { class: "text-gray-600 dark:text-gray-300",
                                    "欢迎, {username}"
                                }
                                button {
                                    class: "px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-colors",
                                    onclick: move |_| {
                                        let nav = navigator.clone();
                                        spawn(async move {
                                            let _ = logout().await;
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let cookie = "session=; path=/; max-age=0";
                                                if let Some(window) = web_sys::window() {
                                                    if let Some(document) = window.document() {
                                                        let _ = document.dyn_into::<web_sys::HtmlDocument>()
                                                            .map(|d| d.set_cookie(cookie));
                                                    }
                                                }
                                            }
                                            let _ = nav.push("/login");
                                        });
                                    },
                                    "登出"
                                }
                            }
                        }
                    }
                    main { class: "max-w-7xl mx-auto px-4 py-8",
                        p { class: "text-gray-600 dark:text-gray-300",
                            "后台管理界面开发中..."
                        }
                    }
                }
            }
        }
        Some(None) => {
            use_effect(move || {
                navigator.push("/login");
            });
            rsx! {
                div { class: "min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900",
                    p { class: "text-gray-600 dark:text-gray-300", "正在跳转..." }
                }
            }
        }
        None => {
            rsx! {
                div { class: "min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900",
                    p { class: "text-gray-600 dark:text-gray-300", "加载中..." }
                }
            }
        }
    }
}

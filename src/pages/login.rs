use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::api::auth::{login, AuthResponse};

#[component]
pub fn LoginPage() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut error = use_signal(|| None::<String>);

    let on_submit = move |_| {
        error.set(None);

        let username_val = username();
        let password_val = password();

        spawn(async move {
            match login(username_val, password_val).await {
                Ok(AuthResponse {
                    success: true,
                    token: Some(_token),
                    ..
                }) => {
                    // 设置 cookie (client-side, not HttpOnly but works for now)
                    #[cfg(target_arch = "wasm32")]
                    {
                        let cookie = format!(
                            "session={}; path=/; max-age={}; SameSite=Lax",
                            _token,
                            30 * 24 * 60 * 60 // 30 days
                        );
                        if let Some(window) = web_sys::window() {
                            if let Some(document) = window.document() {
                                let _ = document
                                    .dyn_into::<web_sys::HtmlDocument>()
                                    .map(|d| d.set_cookie(&cookie));
                            }
                        }
                    }
                    // 跳转到 admin 页面
                    let _ = dioxus::router::navigator().push("/admin");
                }
                Ok(AuthResponse {
                    success: false,
                    message,
                    ..
                }) => {
                    error.set(Some(message));
                }
                Ok(AuthResponse {
                    success: true,
                    token: None,
                    ..
                }) => {
                    error.set(Some("登录异常".to_string()));
                }
                Err(e) => {
                    error.set(Some(format!("请求失败: {}", e)));
                }
            }
        });
    };

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900",
            div { class: "w-full max-w-md p-8 bg-white dark:bg-gray-800 rounded-2xl shadow-lg",
                h1 { class: "text-2xl font-bold text-center text-gray-900 dark:text-white mb-6",
                    "登录"
                }

                if let Some(err) = error() {
                    div { class: "mb-4 p-3 bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300 rounded-lg text-center",
                        "{err}"
                    }
                }

                div { class: "space-y-4",
                    div {
                        label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1",
                            "用户名"
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "text",
                            placeholder: "用户名",
                            value: username(),
                            oninput: move |e| username.set(e.value()),
                        }
                    }
                    div {
                        label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1",
                            "密码"
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "password",
                            placeholder: "密码",
                            value: password(),
                            oninput: move |e| password.set(e.value()),
                        }
                    }
                    button {
                        class: "w-full py-2 px-4 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors",
                        onclick: on_submit,
                        "登录"
                    }
                }
            }
        }
    }
}

use dioxus::prelude::*;

use crate::api::auth::{login, AuthResponse};

#[component]
pub fn LoginPage() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut error = use_signal(|| None::<String>);

    let on_submit = Callback::new(move |_| {
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
    });

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-white dark:bg-[#1d1e20]",
            div { class: "w-full max-w-md p-8 bg-white dark:bg-[#2e2e33] rounded-2xl border border-gray-200 dark:border-[#333]",
                h1 { class: "text-2xl font-bold text-center text-gray-900 dark:text-[#dadadb] mb-6",
                    "登录"
                }

                if let Some(err) = error() {
                    div { class: "mb-4 p-3 bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 rounded-lg text-center",
                        "{err}"
                    }
                }

                div { class: "space-y-4",
                    div {
                        label { class: "block text-sm font-medium text-gray-700 dark:text-[#9b9c9d] mb-1",
                            "用户名 / 邮箱"
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-200 dark:border-[#333] rounded-lg bg-white dark:bg-[#2e2e33] text-gray-900 dark:text-[#dadadb] focus:outline-none focus:border-gray-400 dark:focus:border-gray-600",
                            r#type: "text",
                            placeholder: "用户名或邮箱",
                            value: username(),
                            oninput: move |e| username.set(e.value()),
                            onkeydown: move |e| if e.key() == Key::Enter { on_submit(()) },
                        }
                    }
                    div {
                        label { class: "block text-sm font-medium text-gray-700 dark:text-[#9b9c9d] mb-1",
                            "密码"
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-200 dark:border-[#333] rounded-lg bg-white dark:bg-[#2e2e33] text-gray-900 dark:text-[#dadadb] focus:outline-none focus:border-gray-400 dark:focus:border-gray-600",
                            r#type: "password",
                            placeholder: "密码",
                            value: password(),
                            oninput: move |e| password.set(e.value()),
                            onkeydown: move |e| if e.key() == Key::Enter { on_submit(()) },
                        }
                    }
                    button {
                        class: "w-full py-2 px-4 bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 font-medium rounded-full hover:opacity-80 transition-opacity",
                        onclick: move |_| on_submit(()),
                        "登录"
                    }
                    button {
                        class: "block w-full py-2 px-4 text-center text-gray-500 dark:text-[#9b9c9d] hover:text-gray-700 dark:hover:text-[#dadadb] font-medium rounded-lg transition-colors cursor-pointer",
                        onclick: move |_| { dioxus::router::navigator().push("/register"); },
                        "还没有账号？去注册"
                    }
                }
            }
        }
    }
}

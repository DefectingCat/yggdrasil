use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::auth::{login, AuthResponse};
use crate::components::forms::{AlertBox, FormInput, FormLabel, BUTTON_PRIMARY_CLASS};
use crate::router::Route;

#[component]
pub fn Login() -> Element {
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
                    let _ = dioxus::router::navigator().push(Route::Admin {});
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
                    AlertBox { message: err, variant: "error" }
                }

                div { class: "space-y-4",
                    div {
                        FormLabel { label: "用户名 / 邮箱" }
                        FormInput {
                            r#type: "text",
                            placeholder: "用户名或邮箱",
                            value: username(),
                            oninput: move |v: String| username.set(v),
                            onkeydown: Some(EventHandler::new(move |e: KeyboardEvent| if e.key() == Key::Enter { on_submit(()) })),
                        }
                    }
                    div {
                        FormLabel { label: "密码" }
                        FormInput {
                            r#type: "password",
                            placeholder: "密码",
                            value: password(),
                            oninput: move |v: String| password.set(v),
                            onkeydown: Some(EventHandler::new(move |e: KeyboardEvent| if e.key() == Key::Enter { on_submit(()) })),
                        }
                    }
                    button {
                        class: "{BUTTON_PRIMARY_CLASS}",
                        onclick: move |_| on_submit(()),
                        "登录"
                    }
                    Link {
                        class: "block w-full py-2 px-4 text-center text-gray-500 dark:text-[#9b9c9d] hover:text-gray-700 dark:hover:text-[#dadadb] font-medium rounded-lg transition-colors cursor-pointer",
                        to: Route::Register {},
                        "还没有账号？去注册"
                    }
                }
            }
        }
    }
}

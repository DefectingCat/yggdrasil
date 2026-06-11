use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::auth::{login, AuthResponse};
use crate::components::forms::{AlertBox, FormInput, FormLabel, BUTTON_PRIMARY_CLASS};
use crate::context::UserContext;
use crate::router::Route;

#[component]
pub fn Login() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut ctx: UserContext = use_context();

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
                    ctx.checked.set(false);
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
        div { class: "min-h-screen flex items-center justify-center bg-paper-theme",
            div { class: "w-full max-w-md p-8 bg-paper-entry rounded-2xl border border-paper-border shadow-sm",
                h1 { class: "text-2xl font-bold text-center text-paper-primary mb-6",
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
                        class: "block w-full py-2 px-4 text-center text-paper-secondary hover:text-paper-accent font-medium rounded-lg transition-all duration-200 cursor-pointer",
                        to: Route::Register {},
                        "还没有账号？去注册"
                    }
                }
            }
        }
    }
}

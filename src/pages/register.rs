use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::auth::{register, AuthResponse};
use crate::components::forms::{AlertBox, FormInput, FormLabel, BUTTON_PRIMARY_CLASS};
use crate::router::Route;

#[component]
pub fn Register() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut email = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut confirm_password = use_signal(|| "".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);
    let mut loading = use_signal(|| false);

    let on_submit = Callback::new(move |_| {
        if loading() {
            return;
        }
        error.set(None);
        success.set(false);

        if password().len() < 8 {
            error.set(Some("密码长度至少 8 位".to_string()));
            return;
        }
        if password() != confirm_password() {
            error.set(Some("两次输入的密码不一致".to_string()));
            return;
        }

        let username_val = username();
        let email_val = email();
        let password_val = password();

        loading.set(true);

        spawn(async move {
            match register(username_val, email_val, password_val).await {
                Ok(AuthResponse { success: true, .. }) => {
                    success.set(true);
                }
                Ok(AuthResponse {
                    success: false,
                    message,
                    ..
                }) => {
                    error.set(Some(message));
                }
                Err(e) => {
                    error.set(Some(format!("请求失败: {}", e)));
                }
            }
            loading.set(false);
        });
    });

    let is_loading = loading();

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-paper-theme",
            div { class: "w-full max-w-md p-8 bg-paper-entry rounded-2xl border border-paper-border shadow-sm",
                h1 { class: "text-2xl font-bold text-center text-paper-primary mb-2",
                    "注册"
                }
                p { class: "text-sm text-center text-paper-secondary mb-6",
                    "首个注册账号将自动成为管理员"
                }

                if success() {
                    div { class: "mb-4 p-3 bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded-lg text-center",
                        "注册成功！"
                        Link { class: "block mt-2 text-paper-accent hover:underline cursor-pointer",
                            to: Route::Login {},
                            "去登录"
                        }
                    }
                }

                if let Some(err) = error() {
                    AlertBox { message: err, variant: "error" }
                }

                div { class: "space-y-4",
                    div {
                        FormLabel { label: "用户名" }
                        FormInput {
                            r#type: "text",
                            placeholder: "3-50 位字符",
                            value: username(),
                            disabled: is_loading,
                            oninput: move |v: String| username.set(v),
                            onkeydown: None,
                        }
                    }
                    div {
                        FormLabel { label: "邮箱" }
                        FormInput {
                            r#type: "email",
                            placeholder: "your@email.com",
                            value: email(),
                            disabled: is_loading,
                            oninput: move |v: String| email.set(v),
                            onkeydown: None,
                        }
                    }
                    div {
                        FormLabel { label: "密码" }
                        FormInput {
                            r#type: "password",
                            placeholder: "至少 8 位",
                            value: password(),
                            disabled: is_loading,
                            oninput: move |v: String| password.set(v),
                            onkeydown: None,
                        }
                    }
                    div {
                        FormLabel { label: "确认密码" }
                        FormInput {
                            r#type: "password",
                            placeholder: "再次输入密码",
                            value: confirm_password(),
                            disabled: is_loading,
                            oninput: move |v: String| confirm_password.set(v),
                            onkeydown: None,
                        }
                    }
                    button {
                        class: "{BUTTON_PRIMARY_CLASS}",
                        class: if is_loading { "opacity-60 cursor-not-allowed" },
                        disabled: is_loading,
                        onclick: move |_| on_submit(()),
                        if is_loading { "注册中..." } else { "注册" }
                    }
                }
                p { class: "mt-4 text-center text-sm text-paper-secondary",
                    "已有账号？"
                    Link { class: "text-paper-accent hover:underline cursor-pointer",
                        to: Route::Login {},
                        "去登录"
                    }
                }
            }
        }
    }
}

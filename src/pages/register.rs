//! 注册页面
//!
//! 提供新用户注册表单。首个注册成功的用户将自动成为管理员，
//! 后续注册请求会被服务端拒绝。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::auth::{register, AuthResponse};
use crate::components::forms::{AlertBox, FormInput, FormLabel, BUTTON_PRIMARY_CLASS};
use crate::router::Route;

/// 注册页面组件
#[component]
pub fn Register() -> Element {
    // 表单输入状态
    let mut username = use_signal(|| "".to_string());
    let mut email = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut confirm_password = use_signal(|| "".to_string());
    // 错误提示、成功提示与加载状态
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);
    let mut loading = use_signal(|| false);

    // 提交注册表单
    let on_submit = Callback::new(move |_| {
        if loading() {
            return;
        }
        error.set(None);
        success.set(false);

        // 前端基础校验：密码长度与一致性
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

        // 在异步任务中调用 server function 注册
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
                        Link {
                            class: "block mt-2 text-paper-accent hover:underline cursor-pointer",
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
                        FormLabel {
                            label: "用户名",
                            html_for: Some("register-username".to_string()),
                        }
                        FormInput {
                            id: Some("register-username".to_string()),
                            r#type: "text",
                            placeholder: "3-50 位字符",
                            value: username(),
                            disabled: is_loading,
                            oninput: move |v: String| username.set(v),
                            // 回车键触发提交
                            onkeydown: Some(
                                EventHandler::new(move |e: KeyboardEvent| {
                                    if e.key() == Key::Enter {
                                        on_submit(())
                                    }
                                }),
                            ),
                        }
                    }
                    div {
                        FormLabel {
                            label: "邮箱",
                            html_for: Some("register-email".to_string()),
                        }
                        FormInput {
                            id: Some("register-email".to_string()),
                            r#type: "email",
                            placeholder: "your@email.com",
                            value: email(),
                            disabled: is_loading,
                            oninput: move |v: String| email.set(v),
                            // 回车键触发提交
                            onkeydown: Some(
                                EventHandler::new(move |e: KeyboardEvent| {
                                    if e.key() == Key::Enter {
                                        on_submit(())
                                    }
                                }),
                            ),
                        }
                    }
                    div {
                        FormLabel {
                            label: "密码",
                            html_for: Some("register-password".to_string()),
                        }
                        FormInput {
                            id: Some("register-password".to_string()),
                            r#type: "password",
                            placeholder: "至少 8 位",
                            value: password(),
                            disabled: is_loading,
                            oninput: move |v: String| password.set(v),
                            // 回车键触发提交
                            onkeydown: Some(
                                EventHandler::new(move |e: KeyboardEvent| {
                                    if e.key() == Key::Enter {
                                        on_submit(())
                                    }
                                }),
                            ),
                        }
                    }
                    div {
                        FormLabel {
                            label: "确认密码",
                            html_for: Some("register-confirm-password".to_string()),
                        }
                        FormInput {
                            id: Some("register-confirm-password".to_string()),
                            r#type: "password",
                            placeholder: "再次输入密码",
                            value: confirm_password(),
                            disabled: is_loading,
                            oninput: move |v: String| confirm_password.set(v),
                            // 回车键触发提交
                            onkeydown: Some(
                                EventHandler::new(move |e: KeyboardEvent| {
                                    if e.key() == Key::Enter {
                                        on_submit(())
                                    }
                                }),
                            ),
                        }
                    }
                    button {
                        class: "{BUTTON_PRIMARY_CLASS}",
                        class: if is_loading { "opacity-60 cursor-not-allowed" },
                        disabled: is_loading,
                        onclick: move |_| on_submit(()),
                        if is_loading {
                            "注册中..."
                        } else {
                            "注册"
                        }
                    }
                }
                p { class: "mt-4 text-center text-sm text-paper-secondary",
                    "已有账号？"
                    Link {
                        class: "text-paper-accent hover:underline cursor-pointer",
                        to: Route::Login {},
                        "去登录"
                    }
                }
            }
        }
    }
}

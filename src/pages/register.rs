use dioxus::prelude::*;

use crate::api::auth::{register, AuthResponse};

#[component]
pub fn RegisterPage() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut email = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut confirm_password = use_signal(|| "".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);

    let on_submit = move |_| {
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

        spawn(async move {
            match register(username_val, email_val, password_val).await {
                Ok(AuthResponse { success: true, .. }) => {
                    success.set(true);
                }
                Ok(AuthResponse { success: false, message, .. }) => {
                    error.set(Some(message));
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
                h1 { class: "text-2xl font-bold text-center text-gray-900 dark:text-white mb-2",
                    "注册"
                }
                p { class: "text-sm text-center text-gray-500 dark:text-gray-400 mb-6",
                    "首个注册账号将自动成为管理员"
                }

                if success() {
                    div { class: "mb-4 p-3 bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300 rounded-lg text-center",
                        "注册成功！"
                        a { class: "block mt-2 text-blue-600 dark:text-blue-400 hover:underline", href: "/login",
                            "去登录"
                        }
                    }
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
                            placeholder: "3-50 位字符",
                            value: username(),
                            oninput: move |e| username.set(e.value()),
                        }
                    }
                    div {
                        label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1",
                            "邮箱"
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "email",
                            placeholder: "your@email.com",
                            value: email(),
                            oninput: move |e| email.set(e.value()),
                        }
                    }
                    div {
                        label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1",
                            "密码"
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "password",
                            placeholder: "至少 8 位",
                            value: password(),
                            oninput: move |e| password.set(e.value()),
                        }
                    }
                    div {
                        label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1",
                            "确认密码"
                        }
                        input {
                            class: "w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "password",
                            placeholder: "再次输入密码",
                            value: confirm_password(),
                            oninput: move |e| confirm_password.set(e.value()),
                        }
                    }
                    button {
                        class: "w-full py-2 px-4 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors",
                        onclick: on_submit,
                        "注册"
                    }
                }
                p { class: "mt-4 text-center text-sm text-gray-500 dark:text-gray-400",
                    "已有账号？"
                    a { class: "text-blue-600 dark:text-blue-400 hover:underline", href: "/login",
                        "去登录"
                    }
                }
            }
        }
    }
}

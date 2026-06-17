//! 表单控件组件
//!
//! 提供登录、注册、评论等页面共享的输入框、按钮与提示框样式常量与组件。

use dioxus::prelude::*;

/// 输入框基础 CSS 类，统一文本框、邮箱框、URL 框等样式。
pub const INPUT_CLASS: &str = "w-full px-4 py-2 border border-paper-border rounded-lg bg-paper-entry text-paper-primary placeholder:text-paper-tertiary focus:outline-none focus:border-paper-accent focus:ring-1 focus:ring-paper-accent/30 transition-colors duration-200";

/// 主按钮 CSS 类，用于表单提交等主操作按钮。
pub const BUTTON_PRIMARY_CLASS: &str = "w-full py-2.5 px-4 bg-paper-accent text-white font-medium rounded-full hover:brightness-110 active:scale-[0.98] transition-all duration-200 cursor-pointer";

/// 表单输入框组件。
///
/// Props：
/// - `id`：input 元素 id，用于与 label 关联
/// - `r#type`：input 类型（如 `"text"`、`"email"`、`"password"`）
/// - `placeholder`：占位提示文本
/// - `value`：当前值
/// - `disabled`：是否禁用
/// - `oninput`：输入事件回调，返回新的字符串值
/// - `onkeydown`：可选的键盘事件回调
#[component]
pub fn FormInput(
    id: Option<String>,
    r#type: &'static str,
    placeholder: &'static str,
    value: String,
    disabled: bool,
    oninput: EventHandler<String>,
    onkeydown: Option<EventHandler<KeyboardEvent>>,
) -> Element {
    let disabled_class = if disabled {
        "opacity-60 cursor-not-allowed"
    } else {
        ""
    };
    rsx! {
        input {
            id: id.unwrap_or_default(),
            class: "{INPUT_CLASS} {disabled_class}",
            r#type: "{r#type}",
            placeholder: "{placeholder}",
            value: "{value}",
            disabled,
            oninput: move |e| oninput.call(e.value()),
            onkeydown: move |e| {
                if let Some(ref handler) = onkeydown {
                    handler.call(e);
                }
            },
        }
    }
}

/// 表单标签组件。
///
/// Props：
/// - `label`：标签文本
/// - `html_for`：关联的 input id
#[component]
pub fn FormLabel(label: &'static str, html_for: Option<String>) -> Element {
    rsx! {
        label {
            class: "block text-sm font-medium text-paper-secondary mb-1",
            r#for: html_for.unwrap_or_default(),
            "{label}"
        }
    }
}

/// 提示框组件，用于显示成功、错误等状态消息。
///
/// Props：
/// - `message`：提示文本
/// - `variant`：风格类型，支持 `"error"`、`"success"` 与其他默认类型
#[component]
pub fn AlertBox(message: String, variant: &'static str) -> Element {
    let (bg_class, text_class) = match variant {
        "error" => (
            "bg-red-100 dark:bg-red-900/30",
            "text-red-700 dark:text-red-300",
        ),
        "success" => (
            "bg-green-100 dark:bg-green-900/30",
            "text-green-700 dark:text-green-300",
        ),
        _ => ("bg-paper-code-bg", "text-paper-secondary"),
    };
    rsx! {
        div { class: "mb-4 p-3 {bg_class} {text_class} rounded-lg text-center",
            "{message}"
        }
    }
}

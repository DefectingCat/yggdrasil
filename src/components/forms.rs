//! 表单控件组件
//!
//! 提供登录、注册、评论等页面共享的输入框、按钮与提示框样式常量与组件。

use dioxus::prelude::*;

/// 输入框基础 CSS 类，统一文本框、邮箱框、URL 框等样式。
pub const INPUT_CLASS: &str = "w-full px-4 py-2 border border-paper-border rounded-2xl bg-paper-entry text-paper-primary placeholder:text-paper-tertiary focus:outline-none focus:border-paper-accent focus:ring-1 focus:ring-paper-accent/30 transition-colors duration-200";

/// 主按钮 CSS 类，用于表单提交等主操作按钮。
pub const BUTTON_PRIMARY_CLASS: &str = "w-full py-2.5 px-4 bg-paper-accent text-white font-medium rounded-full hover:brightness-110 active:scale-[0.98] transition-all duration-200 cursor-pointer";

/// 下拉选择框组件（自定义 chevron，跨浏览器对齐一致）。
///
/// 原生 `<select>` 的默认箭头由各浏览器自行绘制，在深色主题与圆角样式下
/// 位置/颜色不协调（无法垂直居中、无法跟随主题色），故用 `appearance-none`
/// 隐藏后叠一个 `top-1/2 -translate-y-1/2` 垂直居中的 chevron SVG
/// （`pointer-events-none`，点击穿透到 select）。
///
/// 泛型值绑定：`<option>` 的 value 内部用序号占位，`onchange` 直接回传选中
/// 项的 `T`，调用方无需做「字符串反查枚举」的样板代码。
///
/// Props：
/// - `id`：select 元素 id，用于与 label 关联
/// - `value`：当前选中项（受控）
/// - `options`：可选项 `(值, 标签)` 列表
/// - `onchange`：选中变化回调，回传新选中项的值
#[component]
pub fn FormSelect<T: Clone + PartialEq + 'static>(
    id: Option<String>,
    value: T,
    options: Vec<(T, &'static str)>,
    onchange: EventHandler<T>,
) -> Element {
    // 与 INPUT_CLASS 同族，但隐藏原生箭头（appearance-none），
    // 右侧预留 pr-10 空间给自定义 chevron。定义在函数体内：当前仅 wasm 端
    // 后台页面使用本组件，模块级常量在 server 构建下会触发 dead_code。
    const SELECT_CLASS: &str = "w-full appearance-none cursor-pointer truncate pl-4 pr-10 py-2 border border-paper-border rounded-2xl bg-paper-entry text-paper-primary focus:outline-none focus:border-paper-accent focus:ring-1 focus:ring-paper-accent/30 transition-colors duration-200";

    // 受控选中序号；value 不在 options 时兜底 0（与浏览器默认选中第一项一致）。
    let selected = options.iter().position(|(v, _)| *v == value).unwrap_or(0);
    // onchange 闭包与下方 for 循环各需一份 options（序号 → 值回查 / 渲染）。
    let options_for_change = options.clone();
    rsx! {
        div { class: "relative",
            select {
                id: id.unwrap_or_default(),
                class: "{SELECT_CLASS}",
                value: "{selected}",
                onchange: move |e| {
                    if let Ok(i) = e.value().parse::<usize>() {
                        if let Some((v, _)) = options_for_change.get(i) {
                            onchange.call(v.clone());
                        }
                    }
                },
                for (i, (_, option_label)) in options.iter().enumerate() {
                    option { value: "{i}", selected: i == selected, "{option_label}" }
                }
            }
            // 自定义下拉箭头（替代原生箭头，保证垂直居中 + 跟随主题色）
            svg {
                class: "pointer-events-none absolute right-4 top-1/2 -translate-y-1/2 w-4 h-4 text-paper-secondary",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                path {
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    d: "M6 9l6 6 6-6",
                }
            }
        }
    }
}

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
        div { class: "mb-4 p-3 {bg_class} {text_class} rounded-lg text-center", "{message}" }
    }
}

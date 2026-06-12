//! 骨架屏原子组件
//!
//! 提供通用的脉冲动画占位块，供各页面骨架屏组合使用。

use dioxus::prelude::*;

/// 通用骨架占位块。
///
/// Props：
/// - `class`：Tailwind CSS 类，控制尺寸与形状
/// - `style`：可选的内联样式字符串
///
/// 默认带有 `animate-pulse` 动画与半透明的占位背景。
#[component]
pub fn SkeletonBox(class: &'static str, style: Option<&'static str>) -> Element {
    rsx! {
        div {
            class: "bg-paper-tertiary/30 dark:bg-[#5a5a62] animate-pulse {class}",
            style: style.unwrap_or(""),
        }
    }
}

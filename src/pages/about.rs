//! 关于页面模块。
//!
//! 对应路由 `/about`。
//!
//! 该页面为静态展示页面，不发起任何 server function 调用，
//! 直接渲染博客介绍、技术栈与主要特性。

use dioxus::prelude::*;

/// 关于页面组件，对应路由 `/about`。
///
/// 展示 Yggdrasil 博客的简介、技术栈与功能特性。
#[component]
pub fn About() -> Element {
    rsx! {
        header { class: "page-header mb-6",
            h1 { class: "text-4xl font-bold text-paper-primary tracking-tight", "关于" }
        }
        article { class: "prose dark:prose-invert max-w-none text-paper-content leading-relaxed",
            p { "世界……遗忘我……" }
        }
    }
}

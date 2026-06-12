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
            h1 { class: "text-4xl font-bold text-paper-primary tracking-tight",
                "关于"
            }
        }
        article { class: "prose dark:prose-invert max-w-none text-paper-content leading-relaxed",
            p { "Yggdrasil 是一个以文字为主的简约博客系统。" }
            p { "它使用 Rust + Dioxus 构建，采用 PostgreSQL 作为数据库，支持 Markdown 写作、标签管理和暗色模式。" }
            h2 { class: "text-xl font-bold text-paper-primary mt-8 mb-4", "技术栈" }
            ul { class: "list-disc pl-5 space-y-1",
                li { "Rust + Dioxus 0.7 (全栈 Web 框架)" }
                li { "PostgreSQL + tokio-postgres (数据库)" }
                li { "Tailwind CSS (样式)" }
                li { "Tiptap Editor (富文本编辑器)" }
                li { "pulldown-cmark (Markdown 渲染)" }
            }
            h2 { class: "text-xl font-bold text-paper-primary mt-8 mb-4", "特性" }
            ul { class: "list-disc pl-5 space-y-1",
                li { "Markdown 写作与实时预览" }
                li { "文章标签与归档" }
                li { "暗色/亮色主题切换" }
                li { "响应式设计" }
                li { "文章搜索" }
            }
        }
    }
}

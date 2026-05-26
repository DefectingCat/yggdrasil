use dioxus::prelude::*;

use crate::theme::ThemeToggle;

#[derive(Clone, PartialEq)]
struct Post {
    title: &'static str,
    summary: &'static str,
    date: &'static str,
    tags: &'static [&'static str],
    slug: &'static str,
}

const POSTS: &[Post] = &[
    Post {
        title: "开始使用 Rust 构建 Web 应用",
        summary: "Rust 作为一门系统级编程语言，近年来在 Web 开发领域也展现出了强大的生命力。本文将介绍如何使用 Rust 和 Dioxus 框架构建现代化的全栈 Web 应用，从项目搭建到部署的完整流程。",
        date: "2026-05-20",
        tags: &["Rust", "Web"],
        slug: "rust-web-app",
    },
    Post {
        title: "Tailwind CSS 的设计理念与实践",
        summary: "Tailwind CSS 是一种实用优先的 CSS 框架，它改变了我们编写样式的方式。通过原子化的工具类，开发者可以快速构建出美观且一致的界面，而无需在 CSS 文件和 HTML 之间来回切换。",
        date: "2026-05-15",
        tags: &["CSS", "前端"],
        slug: "tailwind-css",
    },
    Post {
        title: "PostgreSQL 在 Rust 项目中的最佳实践",
        summary: "数据库是大多数 Web 应用的核心组件。本文探讨如何在 Rust 项目中高效地使用 PostgreSQL，包括连接池管理、异步查询、事务处理以及常见的性能优化技巧。",
        date: "2026-05-10",
        tags: &["数据库", "Rust"],
        slug: "postgresql-rust",
    },
    Post {
        title: "暗色模式的设计思考",
        summary: "暗色模式不仅仅是颜色的反转，它涉及到一整套设计系统的重新思考。从对比度到语义化颜色，暗色模式需要细致的打磨才能提供舒适的阅读体验。",
        date: "2026-05-05",
        tags: &["设计", "UI"],
        slug: "dark-mode-design",
    },
    Post {
        title: "博客系统的架构演进",
        summary: "从一个简单的静态页面到全栈应用，博客系统的架构经历了多次演进。本文记录了 Yggdrasil 博客从设计到实现的思考过程，以及每次迭代背后的决策依据。",
        date: "2026-04-28",
        tags: &["架构", "博客"],
        slug: "blog-architecture",
    },
    Post {
        title: "Dioxus 0.7 新特性一览",
        summary: "Dioxus 0.7 带来了许多令人兴奋的改进，包括更好的全栈支持、改进的路由系统和更流畅的开发体验。让我们一起看看这些新特性如何提升开发效率。",
        date: "2026-04-20",
        tags: &["Rust", "框架"],
        slug: "dioxus-07",
    },
];

#[component]
pub fn HomePage() -> Element {
    rsx! {
        div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20] transition-colors duration-300",
            Header {}
            main { class: "flex-1 w-full max-w-3xl mx-auto px-6 py-6",
                FirstEntry { post: POSTS[0].clone() }
                for post in POSTS.iter().skip(1) {
                    PostEntry { post: post.clone() }
                }
                Pagination {}
            }
            Footer {}
        }
    }
}

#[component]
fn Header() -> Element {
    rsx! {
        header { class: "sticky top-0 z-40 w-full border-b border-gray-200 dark:border-[#333] bg-white/80 dark:bg-[#1d1e20]/80 backdrop-blur-sm",
            nav { class: "max-w-3xl mx-auto px-6 h-[60px] flex items-center justify-between",
                a {
                    class: "text-2xl font-bold text-gray-900 dark:text-[#dadadb] hover:opacity-80 transition-opacity",
                    href: "/",
                    "Yggdrasil"
                }
                div { class: "flex items-center gap-2",
                    ul { class: "hidden md:flex items-center gap-1",
                        NavItem { href: "/", label: "首页", active: true }
                        NavItem { href: "/archives", label: "归档" }
                        NavItem { href: "/tags", label: "标签" }
                        NavItem { href: "/search", label: "搜索" }
                        NavItem { href: "/about", label: "关于" }
                    }
                    ThemeToggle {}
                }
            }
        }
    }
}

#[component]
fn NavItem(href: &'static str, label: &'static str, #[props(default = false)] active: bool) -> Element {
    let base_class = "px-3 py-1 text-base rounded-lg transition-colors";
    let class_str = if active {
        format!("{} font-medium text-gray-900 dark:text-[#dadadb] underline underline-offset-[0.3rem] decoration-2 decoration-gray-900 dark:decoration-[#dadadb]", base_class)
    } else {
        format!("{} text-gray-600 dark:text-[#9b9c9d] hover:text-gray-900 dark:hover:text-[#dadadb]", base_class)
    };

    rsx! {
        li {
            a { class: "{class_str}", href: "{href}", "{label}" }
        }
    }
}

#[component]
fn FirstEntry(post: Post) -> Element {
    let tag_items = post.tags.iter().map(|t| *t).collect::<Vec<_>>();

    rsx! {
        article { class: "mb-10",
            a { class: "block group", href: "/post/{post.slug}",
                h1 { class: "text-[34px] font-bold leading-tight text-gray-900 dark:text-[#dadadb] group-hover:opacity-80 transition-opacity",
                    "{post.title}"
                }
                div { class: "mt-3 text-base text-gray-500 dark:text-[#9b9c9d] leading-relaxed",
                    "{post.summary}"
                }
                div { class: "mt-4 flex items-center gap-3 text-sm text-gray-400 dark:text-[#9b9c9d]",
                    span { "{post.date}" }
                    span { "·" }
                    for (i, tag) in tag_items.iter().enumerate() {
                        if i > 0 {
                            span { "," }
                        }
                        span { "{tag}" }
                    }
                }
            }
        }
    }
}

#[component]
fn PostEntry(post: Post) -> Element {
    let tag_items = post.tags.iter().map(|t| *t).collect::<Vec<_>>();

    rsx! {
        article { class: "relative mb-6 p-6 bg-white dark:bg-[#2e2e33] rounded-lg border border-gray-200 dark:border-[#333] hover:-translate-y-0.5 hover:border-gray-300 dark:hover:border-gray-600 transition-all duration-250",
            a { class: "block group", href: "/post/{post.slug}",
                h2 { class: "text-2xl font-bold leading-tight text-gray-900 dark:text-[#dadadb] group-hover:opacity-80 transition-opacity",
                    "{post.title}"
                }
                div { class: "mt-2 text-sm text-gray-500 dark:text-[#9b9c9d] leading-relaxed line-clamp-2",
                    "{post.summary}"
                }
                div { class: "mt-3 flex items-center gap-3 text-[13px] text-gray-400 dark:text-[#9b9c9d]",
                    span { "{post.date}" }
                    span { "·" }
                    for (i, tag) in tag_items.iter().enumerate() {
                        if i > 0 {
                            span { "," }
                        }
                        span { "{tag}" }
                    }
                }
            }
        }
    }
}

#[component]
fn Pagination() -> Element {
    rsx! {
        nav { class: "flex mt-10 mb-6",
            a {
                class: "ml-auto inline-flex items-center px-4 py-2 text-sm text-white bg-gray-900 dark:bg-[#dadadb] dark:text-gray-900 rounded-full hover:opacity-80 transition-opacity",
                href: "/page/2",
                "下一页"
                span { class: "ml-1", "»" }
            }
        }
    }
}

#[component]
fn Footer() -> Element {
    rsx! {
        footer { class: "w-full border-t border-gray-200 dark:border-[#333] mt-auto",
            div { class: "max-w-3xl mx-auto px-6 py-5 flex items-center justify-between text-sm text-gray-400 dark:text-[#9b9c9d]",
                span { "© 2026 Yggdrasil Blog" }
                button {
                    class: "p-2 rounded-full bg-gray-100 dark:bg-[#333] hover:bg-gray-200 dark:hover:bg-[#444] transition-colors",
                    onclick: move |_| scroll_to_top(),
                    "↑"
                }
            }
        }
    }
}

fn scroll_to_top() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            let mut options = web_sys::ScrollToOptions::new();
            options.top(0.0);
            options.behavior(web_sys::ScrollBehavior::Smooth);
            let _ = window.scroll_to_with_scroll_to_options(&options);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = ();
    }
}

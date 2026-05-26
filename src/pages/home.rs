use dioxus::prelude::*;

use crate::router::Route;
use crate::theme::ThemeToggle;

#[derive(Clone, PartialEq)]
pub struct Post {
    pub title: &'static str,
    pub summary: &'static str,
    pub date: &'static str,
    pub tags: &'static [&'static str],
    pub slug: &'static str,
}

pub const POSTS: &[Post] = &[
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
                HomeInfo {}
                for post in POSTS.iter() {
                    PostEntry { post: post.clone() }
                }
                Pagination {}
            }
            Footer {}
        }
    }
}

#[component]
pub fn Header() -> Element {
    let route = use_route::<Route>();

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
                        NavItem { href: "/", label: "首页", route: route.clone() }
                        NavItem { href: "/archives", label: "归档", route: route.clone() }
                        NavItem { href: "/tags", label: "标签", route: route.clone() }
                        NavItem { href: "/search", label: "搜索", route: route.clone() }
                        NavItem { href: "/about", label: "关于", route: route.clone() }
                    }
                    ThemeToggle {}
                }
            }
        }
    }
}

#[component]
pub fn NavItem(href: &'static str, label: &'static str, route: Route) -> Element {
    let is_active = match (href, route) {
        ("/", Route::HomePage {}) => true,
        ("/archives", Route::ArchivesPage {}) => true,
        ("/tags", Route::TagsPage {}) => true,
        ("/tags", Route::TagDetailPage { .. }) => true,
        ("/search", Route::SearchPage {}) => true,
        ("/about", Route::AboutPage {}) => true,
        _ => false,
    };

    let base_class = "px-3 py-1 text-base rounded-lg transition-colors";
    let class_str = if is_active {
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
fn HomeInfo() -> Element {
    rsx! {
        div { class: "mb-10 text-center",
            h1 { class: "text-[34px] font-bold leading-tight text-gray-900 dark:text-[#dadadb]",
                "Yggdrasil"
            }
            p { class: "mt-3 text-base text-gray-500 dark:text-[#9b9c9d] leading-relaxed",
                "以文字为主的简约博客系统"
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
pub fn Footer() -> Element {
    let mut visible = use_signal(|| false);

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                let closure = wasm_bindgen::prelude::Closure::wrap(Box::new(move || {
                    if let Some(w) = web_sys::window() {
                        let threshold = w.inner_height().ok()
                            .and_then(|h| h.as_f64())
                            .unwrap_or(0.0);
                        let scroll_y = w.scroll_y().unwrap_or(0.0);
                        let new_visible = scroll_y > threshold;
                        visible.set(new_visible);
                    }
                }) as Box<dyn FnMut()>);

                let _ = window.add_event_listener_with_callback("scroll", wasm_bindgen::JsCast::unchecked_ref(closure.as_ref()));

                let threshold = window.inner_height().ok()
                    .and_then(|h| h.as_f64())
                    .unwrap_or(0.0);
                let scroll_y = window.scroll_y().unwrap_or(0.0);
                visible.set(scroll_y > threshold);

                closure.forget();
            }
        }
    });

    let link_class = use_memo(move || {
        let base = "p-2 rounded-full cursor-pointer hover:opacity-80 transition-all duration-300 text-gray-600 dark:text-gray-300";
        if visible() {
            format!("{} opacity-100 translate-y-0", base)
        } else {
            format!("{} opacity-0 translate-y-2 pointer-events-none", base)
        }
    });

    rsx! {
        footer { class: "w-full border-t border-gray-200 dark:border-[#333] mt-auto",
            div { class: "max-w-3xl mx-auto px-6 py-5 flex items-center justify-between text-sm text-gray-400 dark:text-[#9b9c9d]",
                span { "© 2026 Yggdrasil Blog" }
                a {
                    class: "{link_class}",
                    href: "#top",
                    aria_label: "go to top",
                    title: "Go to Top (Alt + G)",
                    accesskey: "g",
                    onclick: move |evt| {
                        evt.prevent_default();
                        scroll_to_top();
                    },
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        height: "24px",
                        view_box: "0 -960 960 960",
                        width: "24px",
                        fill: "currentColor",
                        path {
                            d: "m296-224-56-56 240-240 240 240-56 56-184-183-184 183Zm0-240-56-56 240-240 240 240-56 56-184-183-184 183Z",
                        }
                    }
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

            if let Ok(history) = window.history() {
                let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(" "));
            }
        }
    }
}

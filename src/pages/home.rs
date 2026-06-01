use dioxus::prelude::*;

use crate::api::posts::{list_published_posts, PostListResponse};
use crate::components::header::{Header, NavItemConfig};
use crate::components::footer::Footer;
use crate::models::post::Post;
use crate::router::Route;
use crate::theme::ThemeToggle;

#[component]
pub fn Home() -> Element {
    let route = use_route::<Route>();
    let posts_res = use_resource(list_published_posts);

    let nav_items = vec![
        NavItemConfig { href: "/", label: "首页", is_active: matches!(route, Route::Home {}) },
        NavItemConfig { href: "/archives", label: "归档", is_active: matches!(route, Route::Archives {}) },
        NavItemConfig { href: "/tags", label: "标签", is_active: matches!(route, Route::Tags {}) || matches!(route, Route::TagDetail { .. }) },
        NavItemConfig { href: "/search", label: "搜索", is_active: matches!(route, Route::Search {}) },
        NavItemConfig { href: "/about", label: "关于", is_active: matches!(route, Route::About {}) },
    ];

    rsx! {
        div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20] transition-colors duration-300",
            Header { nav_items, right_content: rsx! { ThemeToggle {} } }
            main { class: "flex-1 w-full max-w-3xl mx-auto px-6 py-6",
                HomeInfo {}
                match &*posts_res.read() {
                    Some(Ok(PostListResponse { posts })) => {
                        rsx! {
                            for post in posts.iter() {
                                PostEntry { post: post.clone() }
                            }
                            if posts.is_empty() {
                                div { class: "text-center text-gray-500 dark:text-[#9b9c9d] py-20",
                                    "暂无文章"
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        rsx! {
                            div { class: "text-center text-red-500 dark:text-red-400 py-20",
                                "加载失败: {e}"
                            }
                        }
                    }
                    None => {
                        rsx! {
                            div { class: "space-y-6 py-4",
                                for _ in 0..3 {
                                    div { class: "mb-6 p-6 bg-white dark:bg-[#2e2e33] rounded-lg border border-gray-200 dark:border-[#333] animate-pulse",
                                        div { class: "h-7 w-3/4 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-3" }
                                        div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded mb-2" }
                                        div { class: "h-4 w-2/3 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                    }
                                }
                            }
                        }
                    }
                }
                Pagination {}
            }
            Footer {}
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
    let post_slug = post.slug.clone();
    let date_str = post
        .published_at
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| post.created_at.format("%Y-%m-%d").to_string());

    rsx! {
        article { class: "relative mb-6 p-6 bg-white dark:bg-[#2e2e33] rounded-lg border border-gray-200 dark:border-[#333] hover:-translate-y-0.5 hover:border-gray-300 dark:hover:border-gray-600 transition-all duration-250",
            a {
                class: "block group",
                href: "/post/{post_slug}",
                onclick: move |evt| {
                    evt.prevent_default();
                    dioxus::router::navigator().push(format!("/post/{}", post_slug).as_str());
                },
                h2 { class: "text-2xl font-bold leading-tight text-gray-900 dark:text-[#dadadb] group-hover:opacity-80 transition-opacity",
                    "{post.title}"
                }
                div { class: "mt-2 text-sm text-gray-500 dark:text-[#9b9c9d] leading-relaxed line-clamp-2",
                    "{post.summary.as_deref().unwrap_or(\"\")}"
                }
                div { class: "mt-3 flex items-center gap-3 text-[13px] text-gray-400 dark:text-[#9b9c9d]",
                    span { "{date_str}" }
                    if !post.tags.is_empty() {
                        span { "·" }
                        for tag in post.tags.clone().into_iter() {
                            span {
                                a {
                                    class: "hover:text-gray-600 dark:hover:text-[#dadadb] transition-colors",
                                    href: "/tags/{tag}",
                                    onclick: move |evt| {
                                        evt.prevent_default();
                                        evt.stop_propagation();
                                        dioxus::router::navigator().push(format!("/tags/{}", tag).as_str());
                                    },
                                    "{tag}"
                                }
                            }
                        }
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
            button {
                class: "ml-auto inline-flex items-center px-4 py-2 text-sm text-white bg-gray-900 dark:bg-[#dadadb] dark:text-gray-900 rounded-full hover:opacity-80 transition-opacity cursor-pointer",
                onclick: move |_| { dioxus::router::navigator().push("/page/2"); },
                "下一页"
                span { class: "ml-1", "»" }
            }
        }
    }
}

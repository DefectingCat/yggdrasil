use dioxus::prelude::*;

use crate::api::posts::{get_post_by_slug, SinglePostResponse};
use crate::components::nav::use_nav_items;
use crate::components::page_layout::PageLayout;
use crate::hooks::delayed_loading::use_delayed_loading;
use crate::router::Route;

#[component]
pub fn PostDetail(slug: String) -> Element {
    let route = use_route::<Route>();
    let slug_clone = slug.clone();
    let post_res = use_resource(move || get_post_by_slug(slug_clone.clone()));
    let nav_items = use_nav_items(route);
    let show_skeleton = use_delayed_loading(move || post_res.read().is_none());

    rsx! {
        PageLayout { nav_items,
            match &*post_res.read() {
                Some(Ok(SinglePostResponse { post: Some(post) })) => {
                    let date_str = post.formatted_date();

                    rsx! {
                        article { class: "py-6",
                            header { class: "mb-8",
                                h1 { class: "text-3xl md:text-4xl font-bold text-gray-900 dark:text-[#dadadb] leading-tight",
                                    "{post.title}"
                                }
                                div { class: "mt-4 flex items-center gap-3 text-sm text-gray-500 dark:text-[#9b9c9d]",
                                    span { "{date_str}" }
                                    if !post.tags.is_empty() {
                                        span { "·" }
                                        for tag in post.tags.clone().into_iter() {
                                            a {
                                                class: "hover:text-gray-700 dark:hover:text-[#dadadb] transition-colors",
                                                href: "/tags/{tag}",
                                                onclick: move |evt| {
                                                    evt.prevent_default();
                                                    dioxus::router::navigator().push(format!("/tags/{}", tag).as_str());
                                                },
                                                "{tag}"
                                            }
                                        }
                                    }
                                }
                            }
                            div {
                                class: "prose dark:prose-invert max-w-none text-gray-800 dark:text-[#c9cacc] leading-relaxed",
                                dangerous_inner_html: "{post.content_html.as_deref().unwrap_or(\"\")}"
                            }
                            div { class: "mt-12 pt-6 border-t border-gray-200 dark:border-[#333]",
                                button {
                                    class: "text-sm text-gray-500 dark:text-[#9b9c9d] hover:text-gray-700 dark:hover:text-[#dadadb] transition-colors",
                                    onclick: move |_| {
                                        let _ = dioxus::router::navigator().push("/");
                                    },
                                    "← 返回首页"
                                }
                            }
                        }
                    }
                }
                Some(Ok(SinglePostResponse { post: None })) => {
                    rsx! {
                        div { class: "text-center py-20",
                            h2 { class: "text-2xl font-bold text-gray-900 dark:text-[#dadadb] mb-4",
                                "文章不存在"
                            }
                            p { class: "text-gray-500 dark:text-[#9b9c9d] mb-6",
                                "这篇文章可能已被删除或移动。"
                            }
                            button {
                                class: "px-6 py-2 bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full font-medium hover:opacity-80 transition-opacity",
                                onclick: move |_| {
                                    let _ = dioxus::router::navigator().push("/");
                                },
                                "返回首页"
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
                        div { class: if show_skeleton() { "animate-pulse py-6 space-y-4" } else { "py-6 space-y-4 opacity-0" },
                            div { class: "h-10 w-3/4 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                            div { class: "h-4 w-32 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                            div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded mt-8" }
                            div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                            div { class: "h-4 w-2/3 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                        }
                    }
                }
            }
        }
    }
}

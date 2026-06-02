use dioxus::prelude::*;

use crate::api::posts::{get_posts_by_tag, list_tags, PostListResponse, TagListResponse};
use crate::components::nav::use_nav_items;
use crate::components::page_layout::PageLayout;
use crate::components::post_card::PostCard;
use crate::hooks::delayed_loading::use_delayed_loading;
use crate::router::Route;

#[component]
pub fn Tags() -> Element {
    let route = use_route::<Route>();
    let tags_res = use_resource(list_tags);
    let nav_items = use_nav_items(route);
    let show_skeleton = use_delayed_loading(move || tags_res.read().is_none());

    rsx! {
        PageLayout { nav_items,
            header { class: "page-header mb-6",
                h1 { class: "text-[34px] font-bold text-gray-900 dark:text-[#dadadb]",
                    "标签"
                }
                match &*tags_res.read() {
                    Some(Ok(TagListResponse { tags })) => {
                        let total = tags.iter().map(|t| t.post_count).sum::<i64>();
                        rsx! {
                            div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                                "共 "
                                span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{tags.len()}" }
                                " 个标签，"
                                span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{total}" }
                                " 篇文章"
                            }
                        }
                    }
                    Some(Err(_)) => {
                        rsx! {
                            div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                                "加载失败"
                            }
                        }
                    }
                    None => {
                        rsx! {
                            div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                                "加载中..."
                            }
                        }
                    }
                }
            }
            match &*tags_res.read() {
                Some(Ok(TagListResponse { tags })) => {
                    let tags = tags.clone();
                    rsx! {
                        ul { class: "flex flex-wrap gap-4 mt-6",
                            for tag in tags.into_iter() {
                                li {
                                    a {
                                        class: "inline-flex items-center px-3 py-1.5 text-base font-medium bg-gray-100 dark:bg-[#2e2e33] text-gray-700 dark:text-[#9b9c9d] rounded-lg hover:bg-gray-200 dark:hover:bg-[#333] transition-colors",
                                        href: "/tags/{tag.name}",
                                        onclick: move |evt| {
                                            evt.prevent_default();
                                            dioxus::router::navigator().push(format!("/tags/{}", tag.name).as_str());
                                        },
                                        "{tag.name}"
                                        sup { class: "ml-1 text-sm text-gray-500 dark:text-[#9b9c9d]", "{tag.post_count}" }
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(_)) => {
                    rsx! {
                        div { class: "text-center text-red-500 dark:text-red-400 py-20",
                            "加载失败"
                        }
                    }
                }
                None => {
                    rsx! {
                        div { class: if show_skeleton() { "flex flex-wrap gap-4 mt-6 animate-pulse" } else { "flex flex-wrap gap-4 mt-6 opacity-0" },
                            for _ in 0..8 {
                                div { class: "h-8 w-16 bg-gray-200 dark:bg-[#2a2a2a] rounded-lg" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn TagDetail(tag: String) -> Element {
    let route = use_route::<Route>();
    let tag_clone = tag.clone();
    let posts_res = use_resource(move || get_posts_by_tag(tag_clone.clone()));
    let nav_items = use_nav_items(route);
    let show_skeleton = use_delayed_loading(move || posts_res.read().is_none());

    rsx! {
        PageLayout { nav_items,
            header { class: "page-header mb-6",
                h1 { class: "text-[34px] font-bold text-gray-900 dark:text-[#dadadb]",
                    "{tag}"
                }
                match &*posts_res.read() {
                    Some(Ok(PostListResponse { posts })) => {
                        rsx! {
                            div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                                "共 "
                                span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{posts.len()}" }
                                " 篇文章"
                            }
                        }
                    }
                    Some(Err(_)) => {
                        rsx! {
                            div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                                "加载失败"
                            }
                        }
                    }
                    None => {
                        rsx! {
                            div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                                "加载中..."
                            }
                        }
                    }
                }
            }
            match &*posts_res.read() {
                Some(Ok(PostListResponse { posts })) => {
                    rsx! {
                        for post in posts.iter() {
                            PostCard { post: post.clone() }
                        }
                    }
                }
                Some(Err(_)) => {
                    rsx! {
                        div { class: "text-center text-red-500 dark:text-red-400 py-20",
                            "加载失败"
                        }
                    }
                }
                None => {
                    rsx! {
                        div { class: if show_skeleton() { "space-y-6 py-4 animate-pulse" } else { "space-y-6 py-4 opacity-0" },
                            for _ in 0..3 {
                                div { class: "mb-6 p-6 bg-white dark:bg-[#2e2e33] rounded-lg border border-gray-200 dark:border-[#333]",
                                    div { class: "h-7 w-3/4 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-3" }
                                    div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded mb-2" }
                                    div { class: "h-4 w-2/3 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

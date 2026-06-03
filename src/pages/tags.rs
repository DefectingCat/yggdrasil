use dioxus::prelude::*;

use crate::api::posts::{get_posts_by_tag, list_tags, PostListResponse, TagListResponse};
use crate::components::nav::use_nav_items;
use crate::components::page_layout::PageLayout;
use crate::components::post_card::PostCard;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::tags_skeleton::{TagsSkeleton, TagDetailSkeleton};
use crate::router::Route;

#[component]
pub fn Tags() -> Element {
    let route = use_route::<Route>();
    let nav_items = use_nav_items(route);

    rsx! {
        PageLayout { nav_items,
            header { class: "page-header mb-6",
                h1 { class: "text-[34px] font-bold text-gray-900 dark:text-[#dadadb]",
                    "标签"
                }
            }
            TagsContent {}
        }
    }
}

#[component]
fn TagsContent() -> Element {
    let tags_res = use_server_future(list_tags)?;

    let tags_data = tags_res.read().as_ref().map(|r| match r {
        Ok(TagListResponse { tags }) => Ok(tags.clone()),
        Err(e) => Err(e.to_string()),
    });

    match tags_data {
        Some(Ok(tags)) => {
            let total = tags.iter().map(|t| t.post_count).sum::<i64>();
            rsx! {
                div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                    "共 "
                    span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{tags.len()}" }
                    " 个标签，"
                    span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{total}" }
                    " 篇文章"
                }
                ul { class: "flex flex-wrap gap-4 mt-6",
                    for tag in tags {
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
        _ => {
            rsx! {
                DelayedSkeleton { TagsSkeleton {} }
            }
        }
    }
}

#[component]
pub fn TagDetail(tag: String) -> Element {
    let route = use_route::<Route>();
    let nav_items = use_nav_items(route);

    rsx! {
        PageLayout { nav_items,
            header { class: "page-header mb-6",
                h1 { class: "text-[34px] font-bold text-gray-900 dark:text-[#dadadb]",
                    "{tag}"
                }
            }
            TagDetailContent { tag: tag.clone() }
        }
    }
}

#[component]
fn TagDetailContent(tag: String) -> Element {
    let posts_res = use_server_future(move || get_posts_by_tag(tag.clone()))?;

    let posts_data = posts_res.read().as_ref().map(|r| match r {
        Ok(PostListResponse { posts }) => Ok(posts.clone()),
        Err(e) => Err(e.to_string()),
    });

    match posts_data {
        Some(Ok(posts)) => {
            rsx! {
                div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                    "共 "
                    span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{posts.len()}" }
                    " 篇文章"
                }
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
        _ => {
            rsx! {
                DelayedSkeleton { TagDetailSkeleton {} }
            }
        }
    }
}

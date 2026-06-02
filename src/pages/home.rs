use dioxus::prelude::*;

use crate::api::posts::{list_published_posts, PostListResponse};
use crate::components::nav::use_nav_items;
use crate::components::page_layout::PageLayout;
use crate::components::post_card::PostCard;
use crate::router::Route;

const POSTS_PER_PAGE: i32 = 10;

#[component]
pub fn Home() -> Element {
    rsx! { HomeContent { page: 1 } }
}

#[component]
pub fn HomePage(page: i32) -> Element {
    rsx! { HomeContent { page } }
}

#[component]
fn HomeContent(page: i32) -> Element {
    let route = use_route::<Route>();
    let current_page = page.max(1);
    let posts_res = use_resource(move || list_published_posts(current_page, POSTS_PER_PAGE));
    let nav_items = use_nav_items(route);

    rsx! {
        PageLayout { nav_items,
            HomeInfo {}
            match &*posts_res.read() {
                Some(Ok(PostListResponse { posts })) => {
                    rsx! {
                        for post in posts.iter() {
                            PostCard { post: post.clone() }
                        }
                        if posts.is_empty() {
                            div { class: "text-center text-gray-500 dark:text-[#9b9c9d] py-20",
                                "暂无文章"
                            }
                        }
                        Pagination { current_page, posts: posts.clone() }
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
fn Pagination(current_page: i32, posts: Vec<crate::models::post::Post>) -> Element {
    let has_prev = current_page > 1;
    let has_next = posts.len() >= POSTS_PER_PAGE as usize;

    rsx! {
        nav { class: "flex mt-10 mb-6 justify-between",
            if has_prev {
                button {
                    class: "inline-flex items-center px-4 py-2 text-sm text-white bg-gray-900 dark:bg-[#dadadb] dark:text-gray-900 rounded-full hover:opacity-80 transition-opacity cursor-pointer",
                    onclick: move |_| {
                        let prev = current_page - 1;
                        if prev <= 1 {
                            dioxus::router::navigator().push("/");
                        } else {
                            dioxus::router::navigator().push(format!("/page/{}", prev).as_str());
                        }
                    },
                    span { class: "mr-1", "«" }
                    "上一页"
                }
            }
            if has_next {
                button {
                    class: "ml-auto inline-flex items-center px-4 py-2 text-sm text-white bg-gray-900 dark:bg-[#dadadb] dark:text-gray-900 rounded-full hover:opacity-80 transition-opacity cursor-pointer",
                    onclick: move |_| {
                        dioxus::router::navigator().push(format!("/page/{}", current_page + 1).as_str());
                    },
                    "下一页"
                    span { class: "ml-1", "»" }
                }
            }
        }
    }
}

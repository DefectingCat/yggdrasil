use dioxus::prelude::*;

use crate::api::posts::{list_published_posts, PostListResponse};
use crate::components::nav::use_nav_items;
use crate::components::page_layout::PageLayout;
use crate::components::post_card::PostCard;
use crate::components::skeletons::home_skeleton::HomeSkeleton;
use crate::router::Route;

const POSTS_PER_PAGE: i32 = 10;

#[component]
pub fn Home() -> Element {
    rsx! { HomePage { page: 1 } }
}

#[component]
pub fn HomePage(page: i32) -> Element {
    let route = use_route::<Route>();
    let nav_items = use_nav_items(route);
    let current_page = page.max(1);

    rsx! {
        PageLayout { nav_items,
            HomeInfo {}
            SuspenseBoundary {
                fallback: |_| rsx! { HomeSkeleton {} },
                HomePosts { current_page }
            }
        }
    }
}

#[component]
fn HomePosts(current_page: i32) -> Element {
    let posts_res = use_server_future(move || list_published_posts(current_page, POSTS_PER_PAGE))?;

    let posts_data = posts_res.read().as_ref().map(|r| match r {
        Ok(PostListResponse { posts }) => Ok(posts.clone()),
        Err(e) => Err(e.to_string()),
    });

    match posts_data {
        Some(Ok(posts)) => {
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
        _ => {
            rsx! {
                // 骨架屏由 SuspenseBoundary fallback 处理
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

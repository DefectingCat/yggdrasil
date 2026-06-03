use dioxus::prelude::*;

use crate::api::posts::{get_post_stats, list_posts, PostListResponse, PostStatsResponse};
use crate::components::suspense_wrapper::SuspenseWrapper;
use crate::models::post::Post;

#[component]
pub fn Admin() -> Element {
    rsx! {
        div { class: "space-y-8",
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                SuspenseWrapper {
                    StatsSection {}
                }
            }

            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                button {
                    class: "bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full px-6 py-3 text-center font-medium hover:opacity-80 transition-opacity cursor-pointer",
                    onclick: move |_| {
                        dioxus::router::navigator().push("/admin/write");
                    },
                    "写文章"
                }
                button {
                    class: "bg-gray-200 dark:bg-[#333] text-gray-700 dark:text-[#dadadb] rounded-full px-6 py-3 text-center font-medium hover:opacity-80 transition-opacity cursor-pointer",
                    onclick: move |_| {
                        dioxus::router::navigator().push("/admin/posts");
                    },
                    "管理文章"
                }
            }

            div { class: "mb-8",
                h2 { class: "text-xl font-bold text-gray-900 dark:text-[#dadadb] mb-4",
                    "最近文章"
                }
                SuspenseWrapper {
                    RecentPostsSection {}
                }
            }
        }
    }
}

#[component]
fn StatsSection() -> Element {
    let stats_res = use_server_future(get_post_stats)?;

    let stats_data = stats_res.read().as_ref().map(|r| match r {
        Ok(PostStatsResponse { stats }) => Ok(stats.clone()),
        Err(_) => Err(()),
    });

    match stats_data {
        Some(Ok(stats)) => {
            rsx! {
                StatCard { value: stats.total.to_string(), label: "文章总数" }
                StatCard { value: stats.drafts.to_string(), label: "草稿数" }
                StatCard { value: stats.published.to_string(), label: "已发布" }
            }
        }
        Some(Err(_)) => {
            rsx! {
                div { class: "col-span-3 text-center text-red-500 dark:text-red-400 py-6",
                    "加载统计失败"
                }
            }
        }
        _ => {
            rsx! {
                div { class: "col-span-3 text-center text-gray-500 dark:text-[#9b9c9d] py-6",
                    "加载中..."
                }
            }
        }
    }
}

#[component]
fn RecentPostsSection() -> Element {
    let posts_res = use_server_future(list_posts)?;

    let posts_data = posts_res.read().as_ref().map(|r| match r {
        Ok(PostListResponse { posts }) => Ok(posts.clone()),
        Err(_) => Err(()),
    });

    match posts_data {
        Some(Ok(posts)) => {
            rsx! {
                div { class: "space-y-0",
                    for post in posts.iter().take(5) {
                        RecentPostItem { post: post.clone() }
                    }
                }
            }
        }
        Some(Err(_)) => {
            rsx! {
                div { class: "text-center text-red-500 dark:text-red-400 py-6",
                    "加载文章列表失败"
                }
            }
        }
        _ => {
            rsx! {
                div { class: "text-center text-gray-500 dark:text-[#9b9c9d] py-6",
                    "加载中..."
                }
            }
        }
    }
}

#[component]
fn StatCard(value: String, label: String) -> Element {
    rsx! {
        div { class: "rounded-xl bg-white dark:bg-[#2e2e33] border border-gray-200 dark:border-[#333] p-6 text-center",
            div { class: "text-3xl font-bold text-gray-900 dark:text-[#dadadb]",
                "{value}"
            }
            div { class: "text-sm text-gray-500 dark:text-[#9b9c9d] mt-2",
                "{label}"
            }
        }
    }
}

#[component]
fn RecentPostItem(post: Post) -> Element {
    let date_str = post
        .published_at
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| post.created_at.format("%Y-%m-%d").to_string());
    let status_label = if post.status == crate::models::post::PostStatus::Published {
        "已发布"
    } else {
        "草稿"
    };
    let status_class = if post.status == crate::models::post::PostStatus::Published {
        "text-green-600 dark:text-green-400"
    } else {
        "text-gray-400 dark:text-[#9b9c9d]"
    };

    rsx! {
        div { class: "flex justify-between items-center py-3 border-b border-gray-100 dark:border-[#333]",
            div { class: "flex items-center gap-3",
                span { class: "text-gray-700 dark:text-[#dadadb]",
                    "{post.title}"
                }
                span { class: "text-xs {status_class}",
                    "{status_label}"
                }
            }
            span { class: "text-sm text-gray-400 dark:text-[#9b9c9d]",
                "{date_str}"
            }
        }
    }
}

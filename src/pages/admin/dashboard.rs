use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::posts::{get_post_stats, list_posts, PostListResponse, PostStatsResponse};
use crate::hooks::delayed_loading::use_delayed_loading;
use crate::models::post::Post;
use crate::router::Route;

#[component]
pub fn Admin() -> Element {
    let stats_res = use_resource(get_post_stats);
    let posts_res = use_resource(list_posts);
    let show_stats_skeleton = use_delayed_loading(move || stats_res.read().is_none());
    let show_posts_skeleton = use_delayed_loading(move || posts_res.read().is_none());

    rsx! {
        div { class: "space-y-8",
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                match &*stats_res.read() {
                    Some(Ok(PostStatsResponse { stats })) => {
                        rsx! {
                            StatCard { value: stats.total.to_string(), label: "文章总数" }
                            StatCard { value: stats.drafts.to_string(), label: "草稿数" }
                            StatCard { value: stats.published.to_string(), label: "已发布" }
                        }
                    }
                    _ => {
                        rsx! {
                            for _ in 0..3 {
                                div { class: if show_stats_skeleton() { "rounded-xl bg-white dark:bg-[#2e2e33] border border-gray-200 dark:border-[#333] p-6 text-center space-y-3 animate-pulse" } else { "rounded-xl bg-white dark:bg-[#2e2e33] border border-gray-200 dark:border-[#333] p-6 text-center space-y-3 opacity-0" },
                                    div { class: "h-9 w-16 mx-auto bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                    div { class: "h-4 w-20 mx-auto bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                Link {
                    class: "bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full px-6 py-3 text-center font-medium hover:opacity-80 transition-opacity cursor-pointer",
                    to: Route::Write {},
                    "写文章"
                }
                Link {
                    class: "bg-gray-200 dark:bg-[#333] text-gray-700 dark:text-[#dadadb] rounded-full px-6 py-3 text-center font-medium hover:opacity-80 transition-opacity cursor-pointer",
                    to: Route::Posts {},
                    "管理文章"
                }
            }

            div { class: "mb-8",
                h2 { class: "text-xl font-bold text-gray-900 dark:text-[#dadadb] mb-4",
                    "最近文章"
                }
                match &*posts_res.read() {
                    Some(Ok(PostListResponse { posts, total: _ })) => {
                        rsx! {
                            div { class: "space-y-0",
                                for post in posts.iter().take(5) {
                                    RecentPostItem { post: post.clone() }
                                }
                            }
                        }
                    }
                    Some(Err(_e)) => {
                        rsx! {
                            div { class: "text-center text-red-500 dark:text-red-400 py-20",
                                "加载失败"
                            }
                        }
                    }
                    None => {
                        rsx! {
                            div { class: if show_posts_skeleton() { "space-y-4 animate-pulse" } else { "space-y-4 opacity-0" },
                                for _ in 0..5 {
                                    div { class: "flex justify-between items-center py-3 border-b border-gray-100 dark:border-[#333]",
                                        div { class: "h-4 w-[45%] bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                        div { class: "h-3 w-20 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                    }
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
    let date_str = post.formatted_date();
    let status_label = post.status_label();
    let status_class = post.status_class();

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

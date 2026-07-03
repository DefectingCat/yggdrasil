//! 管理后台仪表盘页面。
//!
//! 采用高密度工业风设计的管理面板，突出核心数据指标与最新的工作流状态。
//! 数据仅在 WASM 前端通过 Dioxus server functions 异步加载。

use dioxus::prelude::*;
use dioxus::router::components::Link;

#[cfg(target_arch = "wasm32")]
use crate::api::comments::get_pending_count;
#[cfg(target_arch = "wasm32")]
use crate::api::posts::{get_post_stats, list_posts};
#[cfg(target_arch = "wasm32")]
use crate::api::posts::{PostListResponse, PostStatsResponse};
use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::ui::{ADMIN_CARD_CLASS, BTN_SECONDARY};
use crate::models::post::{PostListItem, PostStats};
use crate::router::Route;

#[component]
#[allow(unused_mut)]
pub fn Admin() -> Element {
    let mut stats = use_signal(|| None::<PostStats>);
    let mut recent_posts = use_signal(|| None::<Vec<PostListItem>>);
    let mut pending_count = use_signal(|| None::<i64>);
    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if !loaded() {
            loaded.set(true);
            #[cfg(target_arch = "wasm32")]
            {
                spawn(async move {
                    if let Ok(PostStatsResponse { stats: s }) = get_post_stats().await {
                        stats.set(Some(s));
                    }
                });
                spawn(async move {
                    if let Ok(PostListResponse { posts, total: _ }) = list_posts(1, 5).await {
                        recent_posts.set(Some(posts));
                    }
                });
                spawn(async move {
                    if let Ok(resp) = get_pending_count().await {
                        pending_count.set(Some(resp.count));
                    }
                });
            }
        }
    });

    rsx! {
        div { class: "w-full max-w-7xl mx-auto space-y-8",
            // 顶部标题和全局操作栏
            div { class: "flex flex-col md:flex-row md:items-end justify-between gap-6 pb-6 border-b border-paper-border",
                div {
                    h1 { class: "text-2xl font-semibold tracking-tight text-paper-primary", "SYSTEM_DASHBOARD" }
                    p { class: "text-sm text-paper-secondary mt-1 font-mono uppercase tracking-widest", "Overview & Recent Activities" }
                }
                div { class: "flex items-center gap-3",
                    Link {
                        class: "{BTN_SECONDARY}",
                        to: Route::Posts {},
                        "MANAGE_POSTS"
                    }
                    Link {
                        class: "px-6 py-3 rounded-sm text-xs font-mono uppercase tracking-widest text-paper-theme bg-paper-primary hover:bg-paper-primary/90 transition-all cursor-pointer",
                        to: Route::Write {},
                        "+ NEW_POST"
                    }
                }
            }

            // 数据指标 Bento Grid
            div { class: "grid grid-cols-1 md:grid-cols-4 gap-4",
                match stats() {
                    Some(s) => {
                        rsx! {
                            StatCard { value: s.total.to_string(), label: "TOTAL_POSTS", trend: "+12%" }
                            StatCard { value: s.published.to_string(), label: "PUBLISHED", trend: "Active" }
                            StatCard { value: s.drafts.to_string(), label: "DRAFTS", trend: "Pending" }
                        }
                    }
                    None => {
                        rsx! {
                            for _ in 0..3 {
                                div { class: "{ADMIN_CARD_CLASS} p-6 flex flex-col justify-between h-32 animate-pulse",
                                    SkeletonBox { class: "h-3 w-20 rounded" }
                                    SkeletonBox { class: "h-10 w-16 rounded mt-4" }
                                }
                            }
                        }
                    }
                }

                // 评论待办卡片 (独立色块突出)
                Link {
                    class: "block {ADMIN_CARD_CLASS} p-6 bg-paper-entry hover:bg-paper-entry/80 transition-colors h-32 flex flex-col justify-between group",
                    to: Route::AdminComments {},
                    match pending_count() {
                        Some(count) => {
                            let (color_class, text_class) = if count > 0 {
                                ("text-amber-500", "text-amber-500")
                            } else {
                                ("text-paper-secondary", "text-paper-primary")
                            };
                            rsx! {
                                div { class: "text-[11px] font-mono tracking-widest uppercase {color_class}", "PENDING_COMMENTS" }
                                div { class: "flex items-baseline justify-between",
                                    div { class: "text-4xl font-light tracking-tight {text_class}", "{count}" }
                                    div { class: "text-xs font-mono text-paper-secondary group-hover:text-paper-primary transition-colors", "REVIEW ->" }
                                }
                            }
                        }
                        None => {
                            rsx! {
                                SkeletonBox { class: "h-3 w-24 rounded" }
                                SkeletonBox { class: "h-10 w-16 rounded mt-4" }
                            }
                        }
                    }
                }
            }

            // 最近文章列表 (紧凑表格样式)
            div { class: "mt-8",
                div { class: "flex items-center justify-between mb-4",
                    h2 { class: "text-sm font-mono tracking-widest text-paper-secondary uppercase", "RECENT_PUBLICATIONS" }
                }
                div { class: "{ADMIN_CARD_CLASS} overflow-hidden",
                    match recent_posts() {
                        Some(posts) => {
                            rsx! {
                                div { class: "divide-y divide-paper-border",
                                    for post in posts.iter().take(5) {
                                        RecentPostItem { key: "{post.id}", post: post.clone() }
                                    }
                                }
                            }
                        }
                        None => {
                            rsx! {
                                div { class: "divide-y divide-paper-border animate-pulse",
                                    for _ in 0..5 {
                                        div { class: "flex justify-between items-center px-6 py-4",
                                            SkeletonBox { class: "h-4 w-[40%] rounded" }
                                            SkeletonBox { class: "h-3 w-24 rounded" }
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
}

#[component]
fn StatCard(value: String, label: String, trend: String) -> Element {
    rsx! {
        div { class: "{ADMIN_CARD_CLASS} p-6 flex flex-col justify-between h-32 relative overflow-hidden group",
            div { class: "absolute top-0 left-0 w-1 h-full bg-paper-border group-hover:bg-paper-primary transition-colors" }
            div { class: "flex justify-between items-start pl-2",
                div { class: "text-[11px] font-mono tracking-widest text-paper-secondary uppercase", "{label}" }
                div { class: "text-[10px] font-mono px-1.5 py-0.5 rounded-sm border border-paper-border text-paper-tertiary", "{trend}" }
            }
            div { class: "text-4xl font-light tracking-tight text-paper-primary pl-2 mt-4", "{value}" }
        }
    }
}

#[component]
fn RecentPostItem(post: PostListItem) -> Element {
    let date_str = post.formatted_date();
    let status_label = post.status_label();
    // 把圆角的 badge 换成控制台风格的方角 badge
    let status_class = post.status_class().replace("rounded-full", "rounded-sm border border-paper-border");

    rsx! {
        div { class: "flex flex-col sm:flex-row sm:justify-between sm:items-center px-6 py-4 hover:bg-paper-theme transition-colors cursor-pointer group",
            div { class: "flex items-center gap-4",
                span { class: "text-[11px] font-mono text-paper-tertiary w-12 hidden sm:block", "#{post.id:04}" }
                span { class: "text-sm font-medium text-paper-primary group-hover:text-paper-accent transition-colors", "{post.title}" }
                span { class: "text-[10px] font-mono px-2 py-0.5 {status_class} uppercase tracking-wider", "{status_label}" }
            }
            span { class: "text-xs font-mono text-paper-secondary mt-2 sm:mt-0", "{date_str}" }
        }
    }
}

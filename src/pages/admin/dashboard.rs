//! 管理后台仪表盘页面。
//!
//! 展示文章统计、待审核评论数量以及最近文章列表，仅在 WASM 前端通过 Dioxus server functions 加载数据。

use dioxus::prelude::*;
use dioxus::router::components::Link;

// 仅在 WASM 前端使用的 server function 导入。
#[cfg(target_arch = "wasm32")]
use crate::api::comments::get_pending_count;
#[cfg(target_arch = "wasm32")]
use crate::api::posts::{get_post_stats, list_posts};
#[cfg(target_arch = "wasm32")]
use crate::api::posts::{PostListResponse, PostStatsResponse};
use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::ui::{BTN_SECONDARY, ADMIN_CARD_CLASS};
use crate::models::post::{PostListItem, PostStats};
use crate::router::Route;

/// 后台仪表盘页面组件。
///
/// 展示文章总数、草稿数、已发布数的统计卡片，待审核评论入口，以及最近 5 篇文章列表。
/// 所有数据仅在 WASM 前端通过 server functions 异步加载。
#[component]
#[allow(unused_mut)]
pub fn Admin() -> Element {
    // 仪表盘状态：统计数据、最近文章、待审核评论数与首次加载标志。
    let mut stats = use_signal(|| None::<PostStats>);
    let mut recent_posts = use_signal(|| None::<Vec<PostListItem>>);
    let mut pending_count = use_signal(|| None::<i64>);
    let mut loaded = use_signal(|| false);

    // 组件挂载后触发一次：仅在 WASM 前端异步请求仪表盘数据。
    use_effect(move || {
        if !loaded() {
            loaded.set(true);

            // 以下请求只在 WASM 前端执行，SSR 阶段不会访问浏览器 API。
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
        div { class: "space-y-8",
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                match stats() {
                    Some(s) => {
                        rsx! {
                            StatCard { value: s.total.to_string(), label: "文章总数" }
                            StatCard { value: s.drafts.to_string(), label: "草稿数" }
                            StatCard { value: s.published.to_string(), label: "已发布" }
                        }
                    }
                    None => {
                        rsx! {
                            for _ in 0..3 {
                                div { class: "{ADMIN_CARD_CLASS} p-6 text-center space-y-3 animate-pulse",
                                    SkeletonBox { class: "h-9 w-16 mx-auto rounded" }
                                    SkeletonBox { class: "h-4 w-20 mx-auto rounded" }
                                }
                            }
                        }
                    }
                }
            }

            Link {
                class: "block {ADMIN_CARD_CLASS} p-6 text-center hover:border-paper-accent transition-colors",
                to: Route::AdminComments {},
                match pending_count() {
                    Some(count) => {
                        rsx! {
                            div { class: "text-3xl font-bold text-amber-600 dark:text-amber-400",
                                "{count}"
                            }
                            div { class: "text-sm text-paper-secondary mt-2",
                                "待审核评论"
                            }
                        }
                    }
                    None => {
                        rsx! {
                            SkeletonBox { class: "h-9 w-16 mx-auto rounded" }
                            SkeletonBox { class: "h-4 w-20 mx-auto rounded mt-3" }
                        }
                    }
                }
            }

            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                Link {
                    class: "inline-flex items-center justify-center bg-paper-accent text-paper-theme rounded-full px-6 py-3 text-center font-medium hover:brightness-110 active:scale-[0.98] transition-all duration-200 cursor-pointer",
                    to: Route::Write {},
                    "写文章"
                }
                Link {
                    class: "inline-flex items-center justify-center {BTN_SECONDARY}",
                    to: Route::Posts {},
                    "管理文章"
                }
            }

            div { class: "mb-8",
                h2 { class: "text-xl font-bold text-paper-primary mb-4",
                    "最近文章"
                }
                match recent_posts() {
                    Some(posts) => {
                        rsx! {
                            div { class: "space-y-0",
                                for post in posts.iter().take(5) {
                                    RecentPostItem { post: post.clone() }
                                }
                            }
                        }
                    }
                    None => {
                        rsx! {
                            div { class: "space-y-4 animate-pulse",
                                for _ in 0..5 {
                                    div { class: "flex justify-between items-center py-3 border-b border-paper-border",
                                        SkeletonBox { class: "h-4 w-[45%] rounded" }
                                        SkeletonBox { class: "h-3 w-20 rounded" }
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

/// 统计卡片组件，显示一个数值指标与标签。
#[component]
fn StatCard(value: String, label: String) -> Element {
    rsx! {
        div { class: "{ADMIN_CARD_CLASS} p-6 text-center",
            div { class: "text-3xl font-bold text-paper-primary",
                "{value}"
            }
            div { class: "text-sm text-paper-secondary mt-2",
                "{label}"
            }
        }
    }
}

/// 最近文章列表项，显示标题、状态标签与发布日期。
#[component]
fn RecentPostItem(post: PostListItem) -> Element {
    let date_str = post.formatted_date();
    let status_label = post.status_label();
    let status_class = post.status_class();

    rsx! {
        div { class: "flex justify-between items-center py-3 border-b border-paper-border",
            div { class: "flex items-center gap-3",
                span { class: "text-paper-primary",
                    "{post.title}"
                }
                span { class: "text-xs {status_class}",
                    "{status_label}"
                }
            }
            span { class: "text-sm text-paper-secondary",
                "{date_str}"
            }
        }
    }
}

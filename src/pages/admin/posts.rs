//! 文章管理列表页面。
//!
//! 提供文章分页列表、删除单篇文章、重建 content_html 缓存，以及跳转到编辑器的能力。
//! 数据加载与写操作仅在 WASM 前端通过 Dioxus server functions 完成。

use dioxus::prelude::*;
use dioxus::router::components::Link;

// 仅在 WASM 前端使用的分页数据接口。
#[cfg(target_arch = "wasm32")]
use crate::api::posts::list_posts;
#[cfg(target_arch = "wasm32")]
use crate::api::posts::PostListResponse;
use crate::api::posts::{delete_post, rebuild_content_html, CreatePostResponse, RebuildResult};
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::posts_skeleton::PostsSkeleton;
use crate::components::empty_state::{EmptyState, EmptyStateAction};
use crate::components::ui::{Pagination, StatusBadge, ADMIN_ROW_HOVER, ADMIN_TABLE_CLASS, BTN_TEXT_RED};
use crate::models::post::PostListItem;
use crate::router::Route;

/// 每页展示的文章数量。
const POSTS_PER_PAGE: i32 = 20;

/// 文章管理入口组件，默认展示第 1 页。
#[component]
pub fn Posts() -> Element {
    rsx! {
        PostsPage { page: 1 }
    }
}

/// 文章管理分页组件。
///
/// 根据 `page` 参数加载对应页的文章列表，支持删除单篇文章与重建文章 HTML 缓存。
#[component]
pub fn PostsPage(page: i32) -> Element {
    let current_page = page.max(1);
    // 文章列表、总数、加载状态、删除中 ID、重建缓存状态与结果。
    let mut posts = use_signal(Vec::<PostListItem>::new);
    let mut total = use_signal(|| 0_i64);
    let mut loading = use_signal(|| true);
    let mut deleting = use_signal(|| None::<i32>);

    // 页码变化时加载分页数据：WASM 前端请求接口，SSR 直接结束加载。
    use_effect(move || {
        let _ = current_page;

        loading.set(true);
        // 仅在 WASM 前端发起分页请求。
        #[cfg(target_arch = "wasm32")]
        {
            let p = current_page;
            spawn(async move {
                match list_posts(p, POSTS_PER_PAGE).await {
                    Ok(PostListResponse {
                        posts: list,
                        total: t,
                    }) => {
                        posts.set(list);
                        total.set(t);
                    }
                    Err(_) => {}
                }
                loading.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            loading.set(false);
        }
    });

    let get_posts = move || -> Vec<PostListItem> { posts() };

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold text-paper-primary", "文章管理" }
                div { class: "flex items-center gap-3",
                    // 重建缓存工具条（抽取为子组件 RebuildCacheBar，见文件末尾）。
                    RebuildCacheBar {}
                    Link {
                        class: "px-4 py-2 bg-paper-accent text-paper-theme rounded-full text-sm font-medium hover:brightness-110 active:scale-[0.98] transition-all duration-200 cursor-pointer",
                        to: Route::Write {},
                        "+ 写文章"
                    }
                }
            }

            if loading() && posts().is_empty() {
                DelayedSkeleton { PostsSkeleton {} }
            } else if posts().is_empty() {
                EmptyState {
                    title: "暂无文章",
                    description: "还没有创建任何文章，开始写下你的第一篇文字吧。",
                    action: EmptyStateAction {
                        label: "写文章".to_string(),
                        to: Route::Write {},
                    }
                }
            } else {
                div { class: "{ADMIN_TABLE_CLASS}",
                    table { class: "w-full text-sm",
                        thead {
                            tr { class: "border-b border-paper-border text-left text-paper-secondary",
                                th { class: "px-4 py-3 font-medium", "标题" }
                                th { class: "px-4 py-3 font-medium w-24 text-center",
                                    "状态"
                                }
                                th { class: "px-4 py-3 font-medium w-32", "日期" }
                                th { class: "px-4 py-3 font-medium w-24 text-right",
                                    "操作"
                                }
                            }
                        }
                        tbody {
                            for post in get_posts().iter() {
                                PostRow {
                                    key: "{post.id}",
                                    post: post.clone(),
                                    deleting: deleting() == Some(post.id),
                                    // 删除文章：先乐观更新本地列表，再调用 server function，失败时弹出浏览器提示。
                                    on_delete: move |id| {
                                        deleting.set(Some(id));
                                        let id_for_api = id;
                                        posts.with_mut(|list| list.retain(|p| p.id != id));
                                        total.with_mut(|t| *t = t.saturating_sub(1));
                                        spawn(async move {
                                            match delete_post(id_for_api).await {
                                                Ok(CreatePostResponse { success: false, message: _message, .. }) => {
                                                    #[cfg(target_arch = "wasm32")]
                                                    web_sys::window().map(|w| w.alert_with_message(&_message).ok());
                                                }
                                                Err(_e) => {
                                                    #[cfg(target_arch = "wasm32")]
                                                    web_sys::window().map(|w| w.alert_with_message("删除失败").ok());
                                                }
                                                _ => {}
                                            }
                                            deleting.set(None);
                                        });
                                    },
                                }
                            }
                        }
                    }
                }
                Pagination {
                    variant: "admin",
                    current_page,
                    total: total(),
                    per_page: POSTS_PER_PAGE,
                    prev_route: if current_page - 1 <= 1 { Route::Posts {} } else { Route::PostsPage {
                        page: current_page - 1,
                    } },
                    next_route: Route::PostsPage {
                        page: current_page + 1,
                    },
                    unit: "篇",
                }
            }
        }
    }
}

/// 重建内容缓存工具条子组件。
///
/// 封装「重建内容 / 重建全部」两个按钮及其状态：重建中态（`rebuilding`）、
/// 结果消息（`rebuild_result`）、以及 `do_rebuild` 异步闭包。完全自洽，与父组件
/// 无任何状态共享。
///
/// 从 `PostsPage` 抽取以降低 god component 复杂度（见 dioxus-render-purity skill）。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
fn RebuildCacheBar() -> Element {
    let mut rebuilding = use_signal(|| false);
    let mut rebuild_result = use_signal(|| Option::<String>::None);

    // 重建文章渲染缓存：rebuild_all 为 false 时仅重建 content_html 为空的文章，
    // 为 true 时重建所有文章（用于语法/渲染逻辑升级后批量刷新已有内容）。
    let mut do_rebuild = move |rebuild_all: bool| {
        rebuilding.set(true);
        rebuild_result.set(None);
        spawn(async move {
            match rebuild_content_html(rebuild_all).await {
                Ok(RebuildResult { rebuilt, failed, errors }) => {
                    if failed > 0 {
                        let mut msg = format!("已重建 {rebuilt} 篇，失败 {failed} 篇");
                        if let Some(first) = errors.first() {
                            msg.push_str(&format!("\n{first}"));
                        }
                        rebuild_result.set(Some(msg));
                    } else {
                        rebuild_result.set(Some(format!("已重建 {rebuilt} 篇文章")));
                    }
                }
                Err(e) => {
                    rebuild_result.set(Some(format!("失败: {e}")));
                }
            }
            rebuilding.set(false);
        });
    };

    rsx! {
        // 垂直容器：上方按钮行（与父级 + 写文章 同级水平排列），下方重建结果消息。
        div { class: "flex flex-col gap-2",
            div { class: "flex items-center gap-3",
                div { class: "group relative",
                    button {
                        class: if rebuilding() { "px-4 py-2 rounded-full text-sm font-medium cursor-not-allowed text-paper-secondary border border-paper-border" } else { "px-4 py-2 rounded-full text-sm font-medium cursor-pointer text-paper-primary border border-paper-border hover:border-paper-accent hover:text-paper-accent transition-all" },
                        disabled: rebuilding(),
                        onclick: move |_| do_rebuild(false),
                        if rebuilding() {
                            "重建中..."
                        } else {
                            "重建内容"
                        }
                    }
                    div { class: "pointer-events-none absolute top-full left-1/2 -translate-x-1/2 mt-2 px-3 py-1.5 text-xs font-medium whitespace-nowrap rounded-lg opacity-0 group-hover:opacity-100 transition-opacity duration-200 bg-paper-primary text-paper-theme shadow-lg z-50",
                        "重建 content_html 为空的文章渲染缓存"
                    }
                }
                div { class: "group relative",
                    button {
                        class: if rebuilding() { "px-4 py-2 rounded-full text-sm font-medium cursor-not-allowed text-paper-secondary border border-paper-border" } else { "px-4 py-2 rounded-full text-sm font-medium cursor-pointer text-paper-primary border border-paper-border hover:border-paper-accent hover:text-paper-accent transition-all" },
                        disabled: rebuilding(),
                        onclick: move |_| do_rebuild(true),
                        if rebuilding() {
                            "重建中..."
                        } else {
                            "重建全部"
                        }
                    }
                    div { class: "pointer-events-none absolute top-full left-1/2 -translate-x-1/2 mt-2 px-3 py-1.5 text-xs font-medium whitespace-nowrap rounded-lg opacity-0 group-hover:opacity-100 transition-opacity duration-200 bg-paper-primary text-paper-theme shadow-lg z-50",
                        "重建所有文章的渲染缓存（含已有内容）"
                    }
                }
            }
            if let Some(msg) = rebuild_result() {
                div { class: "text-sm text-paper-secondary px-1", "{msg}" }
            }
        }
    }
}

/// 文章表格行组件，展示单篇文章的标题、状态、日期与操作按钮。
#[component]
fn PostRow(post: PostListItem, deleting: bool, on_delete: EventHandler<i32>) -> Element {
    let date_str = post.formatted_date();

    rsx! {
        tr { class: "{ADMIN_ROW_HOVER}",
            td { class: "px-4 py-3",
                Link {
                    class: "text-paper-primary hover:text-paper-accent transition-colors cursor-pointer",
                    to: Route::PostDetail {
                        slug: post.slug.clone(),
                    },
                    "{post.title}"
                }
            }
            td { class: "px-4 py-3 text-center",
                StatusBadge {
                    color_class: post.status_badge_class(),
                    label: post.status_label().to_string(),
                }
            }
            td { class: "px-4 py-3 text-paper-secondary", "{date_str}" }
            td { class: "px-4 py-3 text-right",
                div { class: "flex justify-end gap-3",
                    Link {
                        class: "text-xs text-paper-secondary hover:text-paper-primary transition-colors cursor-pointer",
                        to: Route::WriteEdit { id: post.id },
                        "编辑"
                    }
                    button {
                        class: if deleting { "text-xs text-paper-secondary cursor-not-allowed" } else { BTN_TEXT_RED },
                        disabled: deleting,
                        onclick: move |_| on_delete.call(post.id),
                        if deleting {
                            "删除中..."
                        } else {
                            "删除"
                        }
                    }
                }
            }
        }
    }
}

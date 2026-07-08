//! 文章管理列表页面。
//!
//! 提供文章分页列表、删除单篇文章、重建 content_html 缓存，以及跳转到编辑器的能力。
//! 数据加载与写操作仅在 WASM 前端通过 Dioxus server functions 完成。

use dioxus::prelude::*;
use dioxus::router::components::Link;

// 分页数据接口：list_posts 是 server function，两端都生成（wasm 端为 client stub，
// server 端为真实实现），故无需 cfg。实际请求只在 use_paginated 的 wasm 分支发出。
use crate::api::posts::{list_posts, PostListResponse};
// get_post_stats / PostStatsResponse 仅在 PostsTabs 的 wasm 加载路径使用，
// SSR 下对应 use_effect 分支被裁剪，故允许 unused imports。
#[allow(unused_imports)]
use crate::api::posts::{
    delete_post, get_post_stats, rebuild_content_html, rebuild_post_content_html,
    CreatePostResponse, PostStatsResponse, RebuildResult,
};
use crate::components::empty_state::{EmptyState, EmptyStateAction};
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::posts_skeleton::PostsSkeleton;
use crate::components::ui::{
    Pagination, StatusBadge, Tooltip, ADMIN_ROW_HOVER, ADMIN_TABLE_CLASS, BTN_OUTLINE,
    BTN_PRIMARY, BTN_TEXT_ACCENT, BTN_TEXT_RED, SPINNER_SVG,
};
use crate::hooks::query::use_paginated;
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

    // 分页列表加载（loading / posts / total / error）由 use_paginated 统一管理；
    // 原先吞掉 Err，现向 trash 看齐暴露 error signal（保持一致性）。
    let paginated = use_paginated(
        move || current_page,
        POSTS_PER_PAGE,
        |p, pp| async move {
            list_posts(p, pp)
                .await
                .map(|PostListResponse { posts, total }| (posts, total))
        },
    );
    let mut posts = paginated.items;
    let mut total = paginated.total;
    let loading = paginated.loading;
    let _error = paginated.error;

    // 删除中 / 重建中文章 ID 集合：均由本组件持有（业务逻辑不归 hook 管）。
    // 改为非乐观删除后行会保留至请求完成，可并发点多个删除，故用 HashSet
    // 与 rebuilding 同形，按行通过 contains 判断 loading 态。
    let mut deleting = use_signal(std::collections::HashSet::<i32>::new);
    // 重建中文章 ID 集合：支持多篇文章并发重建（行不会随点击消失，单值会被后点
    // 的覆盖先点的，故用 HashSet），按行通过 contains 判断 loading 态。
    let mut rebuilding = use_signal(std::collections::HashSet::<i32>::new);
    // 重建缓存的状态由本组件持有并下发给 RebuildCacheBar：结果消息也在本组件
    // 渲染（header 与表格之间的独立行），既不撑高 header 触发 items-center 重排，
    // 也不脱离文档流溢进表格。rebuilding 仅按钮态用，不在此渲染。
    // 不加 mut：本组件只读信号并下发，.set() 都在 RebuildCacheBar 的 spawn 块里，
    // 走 Signal 的内部可变性；SSR target 下那些 set 不可见，mut 会触发 unused_mut。
    let batch_rebuilding = use_signal(|| false);
    let rebuild_result = use_signal(|| Option::<String>::None);

    let get_posts = move || -> Vec<PostListItem> { posts() };

    rsx! {
        div { class: "w-full max-w-7xl mx-auto space-y-6",
            div { class: "flex flex-col md:flex-row md:items-end justify-between gap-6 pb-6 border-b border-paper-border mb-6",
                div {
                    h1 { class: "text-4xl font-extrabold tracking-tight text-[var(--color-paper-primary)]", "管理文章" }
                    p { class: "text-base text-[var(--color-paper-secondary)] mt-2", "所有文章及草稿" }
                }
                div { class: "flex items-center gap-3",
                    // 重建缓存工具条（抽取为子组件 RebuildCacheBar，见文件末尾）。
                    RebuildCacheBar {
                        rebuilding: batch_rebuilding,
                        rebuild_result: rebuild_result,
                    }
                    Link {
                        class: "{BTN_PRIMARY}",
                        to: Route::Write {},
                        "发布文章"
                    }
                }
            }

            // tab 栏：文章 / 回收站。URL 驱动（Link 切换路由），回收站带数量角标。
            PostsTabs {}

            // 重建结果消息：独立成行，进入文档流，吃 space-y-6 的正常间距。
            // 既不撑高 header（不在 header 内）触发 items-center 重排，也不脱离流
            // 溢进表格（曾用 absolute top-full mt-2，因 28px > 24px 间距溢出 4px）。
            if let Some(msg) = rebuild_result() {
                div { class: "text-sm text-paper-secondary",
                    "{msg}"
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
                    },
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
                                th { class: "px-4 py-3 font-medium w-44 text-right",
                                    "操作"
                                }
                            }
                        }
                        tbody {
                            for post in get_posts().iter() {
                                PostRow {
                                    key: "{post.id}",
                                    post: post.clone(),
                                    deleting: deleting().contains(&post.id),
                                    rebuilding: rebuilding().contains(&post.id),
                                    // 删除文章：非乐观——先标记 deleting 让按钮显示 loading，
                                    // 服务端成功后才移除行并减总数，失败则保留行并弹出浏览器提示。
                                    on_delete: move |id| {
                                        deleting.write().insert(id);
                                        spawn(async move {
                                            match delete_post(id).await {
                                                Ok(CreatePostResponse { success: true, .. }) => {
                                                    posts.with_mut(|list| list.retain(|p| p.id != id));
                                                    total.with_mut(|t| *t = t.saturating_sub(1));
                                                }
                                                Ok(CreatePostResponse { success: false, message: _message, .. }) => {
                                                    #[cfg(target_arch = "wasm32")]
                                                    web_sys::window().map(|w| w.alert_with_message(&_message).ok());
                                                }
                                                Err(_e) => {
                                                    #[cfg(target_arch = "wasm32")]
                                                    web_sys::window().map(|w| w.alert_with_message("删除失败").ok());
                                                }
                                            }
                                            deleting.write().remove(&id);
                                        });
                                    },
                                    // 重建单篇文章内容：调用 server function 重新渲染 content_html。
                                    // 静默执行，仅按行切换 rebuilding 按钮态，不弹窗。
                                    // 用 HashSet 记录在途 ID，支持多篇并发重建。
                                    on_rebuild: move |id| {
                                        rebuilding.write().insert(id);
                                        spawn(async move {
                                            let _ = rebuild_post_content_html(id).await;
                                            rebuilding.write().remove(&id);
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
/// 封装「重建内容 / 重建全部」两个按钮及其 `do_rebuild` 异步闭包。状态
/// (`rebuilding` / `rebuild_result`) 由父组件 `PostsPage` 持有并下发：
/// 结果消息在父组件渲染为 header 与表格之间的独立行（进入文档流，吃
/// `space-y-6` 的正常间距），既不撑高 header 触发 `items-center` 重排，
/// 也不脱离文档流溢进表格。
///
/// 从 `PostsPage` 抽取以降低 god component 复杂度（见 dioxus-render-purity skill）。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
fn RebuildCacheBar(
    rebuilding: Signal<bool>,
    rebuild_result: Signal<Option<String>>,
) -> Element {
    // 重建文章渲染缓存：rebuild_all 为 false 时仅重建 content_html 为空的文章，
    // 为 true 时重建所有文章（用于语法/渲染逻辑升级后批量刷新已有内容）。
    let mut do_rebuild = move |rebuild_all: bool| {
        rebuilding.set(true);
        rebuild_result.set(None);
        spawn(async move {
            match rebuild_content_html(rebuild_all).await {
                Ok(RebuildResult {
                    rebuilt,
                    failed,
                    errors,
                }) => {
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
        // 仅渲染按钮行本身：结果消息已上提到 PostsPage，作为独立状态行进入文档流。
        div { class: "flex items-center gap-3",
            Tooltip {
                tip: "重建 content_html 为空的文章渲染缓存".to_string(),
                placement: "bottom",
                button {
                    class: if rebuilding() { "relative px-4 py-2 rounded-full text-sm font-medium cursor-not-allowed text-paper-secondary border border-paper-border" } else { BTN_OUTLINE },
                    disabled: rebuilding(),
                    onclick: move |_| do_rebuild(false),
                    span {
                        class: if rebuilding() { "opacity-40" } else { "" },
                        "重建内容"
                    }
                    if rebuilding() {
                        span {
                            class: "absolute inset-0 flex items-center justify-center",
                            dangerous_inner_html: SPINNER_SVG,
                        }
                    }
                }
            }
            Tooltip {
                tip: "重建所有文章的渲染缓存（含已有内容）".to_string(),
                placement: "bottom",
                button {
                    class: if rebuilding() { "relative px-4 py-2 rounded-full text-sm font-medium cursor-not-allowed text-paper-secondary border border-paper-border" } else { BTN_OUTLINE },
                    disabled: rebuilding(),
                    onclick: move |_| do_rebuild(true),
                    span {
                        class: if rebuilding() { "opacity-40" } else { "" },
                        "重建全部"
                    }
                    if rebuilding() {
                        span {
                            class: "absolute inset-0 flex items-center justify-center",
                            dangerous_inner_html: SPINNER_SVG,
                        }
                    }
                }
            }
        }
    }
}

/// 文章表格行组件，展示单篇文章的标题、状态、日期与操作按钮。
#[component]
fn PostRow(
    post: PostListItem,
    deleting: bool,
    rebuilding: bool,
    on_delete: EventHandler<i32>,
    on_rebuild: EventHandler<i32>,
) -> Element {
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
                div { class: "flex justify-end items-center gap-3",
                    Link {
                        class: "text-xs text-paper-secondary hover:text-paper-primary transition-colors cursor-pointer",
                        to: Route::WriteEdit { id: post.id },
                        "编辑"
                    }
                    Tooltip {
                        tip: "重新渲染这篇文章的 HTML".to_string(),
                        button {
                            class: if rebuilding {
                                "relative inline-flex items-center text-xs text-paper-accent cursor-not-allowed"
                            } else {
                                BTN_TEXT_ACCENT
                            },
                            disabled: rebuilding,
                            onclick: move |_| on_rebuild.call(post.id),
                            span {
                                class: if rebuilding { "opacity-40" } else { "" },
                                "重建"
                            }
                            if rebuilding {
                                span {
                                    class: "absolute inset-0 flex items-center justify-center",
                                    dangerous_inner_html: SPINNER_SVG,
                                }
                            }
                        }
                    }
                    button {
                        class: if deleting {
                            "relative inline-flex items-center text-xs text-paper-secondary cursor-not-allowed"
                        } else {
                            BTN_TEXT_RED
                        },
                        disabled: deleting,
                        onclick: move |_| on_delete.call(post.id),
                        span {
                            class: if deleting { "opacity-40" } else { "" },
                            "删除"
                        }
                        if deleting {
                            span {
                                class: "absolute inset-0 flex items-center justify-center",
                                dangerous_inner_html: SPINNER_SVG,
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 文章管理 tab 栏：「全部文章」与「回收站」。
///
/// tab 状态由 URL（当前路由）驱动而非本地 signal：点击即跳转路由，刷新/深链均可直达
/// 对应 tab。回收站 tab 带 `get_post_stats().stats.trash` 数量角标，便于发现待清理文章。
/// 本组件在 `PostsPage`（全部文章）与 `PostsTrashPage`（回收站）共用，故设为 pub。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
pub fn PostsTabs() -> Element {
    let route = use_route::<Route>();
    // 当前 tab：true=回收站（PostsTrash/PostsTrashPage），false=全部文章（Posts/PostsPage）。
    let is_trash = matches!(
        route,
        Route::PostsTrash {} | Route::PostsTrashPage { .. }
    );
    // 回收站数量：仅 WASM 异步拉取，供角标展示。
    let mut trash_count = use_signal(|| Option::<i64>::None);

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        spawn(async move {
            if let Ok(PostStatsResponse { stats }) = get_post_stats().await {
                trash_count.set(Some(stats.trash));
            }
        });
    });

    rsx! {
        div { class: "flex gap-4 border-b border-paper-border",
            Link {
                to: Route::Posts {},
                class: if !is_trash {
                    "px-2 py-3 text-xs font-mono tracking-widest uppercase text-paper-primary transition-colors cursor-pointer border-b-2 border-paper-primary -mb-px"
                } else {
                    "px-2 py-3 text-xs font-mono tracking-widest uppercase text-paper-secondary hover:text-paper-primary transition-colors cursor-pointer border-b-2 border-transparent -mb-px"
                },
                "全部文章"
            }
            Link {
                to: Route::PostsTrash {},
                class: if is_trash {
                    "inline-flex items-center gap-1.5 px-2 py-3 text-xs font-mono tracking-widest uppercase text-paper-primary transition-colors cursor-pointer border-b-2 border-paper-primary -mb-px"
                } else {
                    "inline-flex items-center gap-1.5 px-2 py-3 text-xs font-mono tracking-widest uppercase text-paper-secondary hover:text-paper-primary transition-colors cursor-pointer border-b-2 border-transparent -mb-px"
                },
                "回收站"
                // 数量角标：有数据才显示。0 显示中性灰，>0 用主题强调色提醒。
                if let Some(count) = trash_count() {
                    span {
                        class: if count > 0 {
                            "inline-flex items-center justify-center min-w-[1.25rem] h-5 px-1.5 rounded-full text-[0.625rem] font-semibold normal-case tracking-normal bg-paper-accent-soft text-paper-accent"
                        } else {
                            "inline-flex items-center justify-center min-w-[1.25rem] h-5 px-1.5 rounded-full text-[0.625rem] font-semibold normal-case tracking-normal bg-paper-tertiary text-paper-secondary"
                        },
                        "{count}"
                    }
                }
            }
        }
    }
}

//! 回收站管理页面。
//!
//! 展示已软删除文章，支持恢复、彻底删除、批量操作、一键清空，
//! 以及自动清理配置（启用开关 + 保留天数）。
//! 数据加载与操作仅在 WASM 前端通过 Dioxus server functions 交互。

use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus::router::components::Link;

// 操作类 server function 在 SSR 与 WASM 均需可见（spawn 闭包需类型检查），
// 但部分仅用于 WASM 代码路径，SSR 下触发 unused imports，按项目惯例放行。
#[allow(unused_imports)]
use crate::api::posts::{
    batch_purge_posts, batch_restore_posts, empty_trash, purge_post, restore_post,
};
#[cfg(target_arch = "wasm32")]
use crate::api::posts::{list_deleted_posts, PostListResponse};
#[allow(unused_imports)]
use crate::api::settings::{get_trash_settings, update_trash_settings};
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::models::post::Post;
use crate::models::settings::TrashSettings;
use crate::router::Route;

/// 每页展示的回收站文章数量。
const TRASH_PER_PAGE: i32 = 20;

/// 回收站入口组件，默认展示第 1 页。
#[component]
pub fn Trash() -> Element {
    rsx! { TrashPage { page: 1 } }
}

/// 回收站分页组件。
///
/// 支持单条/批量恢复与彻底删除、一键清空，以及内联自动清理配置。
#[allow(unused_mut, unused_variables)]
#[component]
pub fn TrashPage(page: i32) -> Element {
    let current_page = page.max(1);
    let mut selected_ids: Signal<HashSet<i32>> = use_signal(HashSet::new);
    let mut posts: Signal<Vec<Post>> = use_signal(Vec::new);
    let mut total: Signal<i64> = use_signal(|| 0);
    #[allow(unused_mut)]
    let mut loading: Signal<bool> = use_signal(|| false);
    #[allow(unused_mut)]
    let mut error: Signal<Option<String>> = use_signal(|| None);
    // 自动清理配置（含本地草稿态用于表单输入）。
    let mut settings: Signal<TrashSettings> = use_signal(TrashSettings::default);
    let mut settings_draft_days: Signal<String> = use_signal(|| "30".to_string());
    let mut settings_draft_enabled: Signal<bool> = use_signal(|| false);
    let mut settings_panel_open: Signal<bool> = use_signal(|| false);
    let mut saving_settings: Signal<bool> = use_signal(|| false);
    // 配置只加载一次的标记，避免每次翻页 effect 重复拉取。
    let mut settings_loaded: Signal<bool> = use_signal(|| false);

    // 加载回收站列表；配置仅首次加载。
    use_effect(move || {
        let _ = current_page;
        loading.set(true);
        error.set(None);
        #[cfg(target_arch = "wasm32")]
        {
            let page = current_page;
            spawn(async move {
                match list_deleted_posts(page, TRASH_PER_PAGE).await {
                    Ok(PostListResponse { posts: list, total: t }) => {
                        posts.set(list);
                        total.set(t);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
            if !settings_loaded() {
                settings_loaded.set(true);
                spawn(async move {
                    if let Ok(s) = get_trash_settings().await {
                        settings_draft_days.set(s.retention_days.to_string());
                        settings_draft_enabled.set(s.auto_purge_enabled);
                        settings.set(s);
                    }
                });
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            loading.set(false);
        }
    });

    // 本地移除一篇文章（乐观更新）。
    let mut remove_post = move |id: i32| {
        posts.with_mut(|list| list.retain(|p| p.id != id));
        total.with_mut(|t| *t = t.saturating_sub(1));
        selected_ids.with_mut(|s| {
            s.remove(&id);
        });
    };

    rsx! {
        div { class: "space-y-6",
            // 标题 + 设置开关入口
            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-3",
                    h1 { class: "text-2xl font-bold text-gray-900 dark:text-[#dadadb]", "回收站" }
                    span { class: "text-sm text-gray-500 dark:text-[#9b9c9d]",
                        "共 {total()} 篇"
                    }
                }
                button {
                    class: "px-4 py-2 rounded-full text-sm font-medium cursor-pointer text-gray-700 dark:text-[#b0b0b1] border border-gray-300 dark:border-[#444] hover:border-gray-900 dark:hover:border-[#dadadb] hover:text-gray-900 dark:hover:text-[#dadadb] transition-all flex items-center gap-2",
                    onclick: move |_| settings_panel_open.set(!settings_panel_open()),
                    // 齿轮 SVG（Material Symbols 风格）
                    svg {
                        class: "w-4 h-4",
                        view_box: "0 -960 960 960",
                        fill: "currentColor",
                        path { d: "m370-80-16-128q-13-5-24.5-12T307-235l-119 50L78-375l103-78q-1-7-1-13.5v-27q0-6.5 1-13.5L78-585l110-190 119 50q11-8 23-15t24-12l16-128h220l16 128q13 5 24.5 12t23.5 15l119-50 110 190-103 78q1 7 1 13.5v27q0 6.5-2 13.5l103 78-110 190-118-50q-11 8-23 15t-24 12L590-80H370Zm70-80h79l14-106q31-8 57.5-23.5T639-327l99 41 39-68-86-65q5-14 7-29.5t2-31.5q0-16-2-31.5t-7-29.5l86-65-39-68-99 42q-22-23-48.5-38.5T533-838l-13-106h-79l-14 106q-31 8-57.5 23.5T321-737l-99-41-39 68 86 64q-5 15-7 30t-2 32q0 16 2 31t7 30l-86 65 39 68 99-42q22 23 48.5 38.5T427-276l13 96Zm39-180q54 0 92-38t38-92q0-54-38-92t-92-38q-54 0-92 38t-38 92q0 54 38 92t92 38Z" }
                    }
                    "自动清理设置"
                }
            }

            // 摘要条：当前清理状态
            div { class: "flex items-center gap-4 px-4 py-3 bg-gray-50 dark:bg-[#2a2a2a] rounded-lg",
                div { class: "flex items-center gap-2",
                    {let dot_class = if settings().auto_purge_enabled {
                        "w-2 h-2 rounded-full bg-green-500"
                    } else {
                        "w-2 h-2 rounded-full bg-gray-400 dark:bg-[#666]"
                    };
                    rsx! {
                        div { class: "{dot_class}" }
                    }}
                    span { class: "text-sm text-gray-600 dark:text-[#9b9c9d]",
                        if settings().auto_purge_enabled {
                            "自动清理已开启 · 超过 {settings().retention_days} 天自动删除"
                        } else {
                            "自动清理已关闭"
                        }
                    }
                }
            }

            // 设置面板（可折叠）
            if settings_panel_open() {
                div { class: "bg-white dark:bg-[#2e2e33] rounded-xl border border-gray-200 dark:border-[#333] p-5 space-y-4",
                    div {
                        label { class: "flex items-center justify-between cursor-pointer",
                            span { class: "text-sm text-gray-700 dark:text-[#dadadb]", "启用自动清理" }
                            input {
                                r#type: "checkbox",
                                class: "w-4 h-4 rounded border-gray-300 dark:border-[#555]",
                                checked: settings_draft_enabled(),
                                onchange: move |e| settings_draft_enabled.set(e.checked()),
                            }
                        }
                    }
                    div { class: "flex items-center gap-3",
                        label { class: "text-sm text-gray-700 dark:text-[#dadadb] whitespace-nowrap", "保留天数" }
                        input {
                            r#type: "number",
                            min: "1",
                            max: "365",
                            class: "w-24 px-3 py-1.5 text-sm rounded-lg border border-gray-300 dark:border-[#444] bg-white dark:bg-[#1d1e20] text-gray-900 dark:text-[#dadadb] focus:outline-none focus:border-gray-900 dark:focus:border-[#dadadb]",
                            value: "{settings_draft_days()}",
                            oninput: move |e| settings_draft_days.set(e.value()),
                        }
                        span { class: "text-xs text-gray-400 dark:text-[#666]", "天后自动彻底删除（1–365）" }
                    }
                    div { class: "flex justify-end",
                        button {
                            class: if saving_settings() {
                                "px-4 py-1.5 text-sm font-medium cursor-not-allowed text-gray-400 bg-gray-100 dark:bg-[#2a2a2a] rounded-full"
                            } else {
                                "px-4 py-1.5 text-sm font-medium text-white bg-gray-900 dark:bg-[#dadadb] dark:text-gray-900 rounded-full hover:opacity-80 transition-opacity cursor-pointer"
                            },
                            disabled: saving_settings(),
                            onclick: move |_| {
                                let days: i32 = settings_draft_days().parse().unwrap_or(30);
                                let enabled = settings_draft_enabled();
                                saving_settings.set(true);
                                spawn(async move {
                                    if let Ok(s) = update_trash_settings(enabled, days).await {
                                        settings.set(s);
                                    }
                                    saving_settings.set(false);
                                });
                            },
                            if saving_settings() { "保存中..." } else { "保存设置" }
                        }
                    }
                }
            }

            // 批量操作栏（选中时显示）
            if !selected_ids().is_empty() {
                div { class: "flex items-center gap-3 p-3 bg-gray-50 dark:bg-[#2a2a2a] rounded-lg",
                    span { class: "text-sm text-gray-600 dark:text-[#9b9c9d]",
                        "已选择 {selected_ids().len()} 条"
                    }
                    button {
                        class: "px-3 py-1.5 text-xs font-medium bg-green-600 text-white rounded hover:bg-green-700 transition-colors",
                        onclick: move |_| {
                            let ids: Vec<i32> = selected_ids().iter().copied().collect();
                            spawn(async move {
                                let _ = batch_restore_posts(ids).await;
                            });
                            for id in selected_ids() { remove_post(id); }
                            selected_ids.set(HashSet::new());
                        },
                        "批量恢复"
                    }
                    button {
                        class: "px-3 py-1.5 text-xs font-medium bg-red-600 text-white rounded hover:bg-red-700 transition-colors",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                if web_sys::window()
                                    .and_then(|w| w.confirm_with_message("确定要彻底删除选中的文章吗？此操作不可恢复。").ok())
                                    .unwrap_or(false)
                                {
                                    let ids: Vec<i32> = selected_ids().iter().copied().collect();
                                    spawn(async move {
                                        let _ = batch_purge_posts(ids).await;
                                    });
                                    for id in selected_ids() { remove_post(id); }
                                    selected_ids.set(HashSet::new());
                                }
                            }
                        },
                        "批量彻底删除"
                    }
                }
            }

            // 主内容：错误 / 加载骨架 / 空态 / 列表
            {
                if error().is_some() {
                    rsx! {
                        div { class: "text-center text-red-500 dark:text-red-400 py-20", "加载失败" }
                    }
                } else if loading() && posts().is_empty() {
                    rsx! {
                        DelayedSkeleton {
                            div { class: "bg-white dark:bg-[#2e2e33] rounded-xl border border-gray-200 dark:border-[#333] p-6 space-y-4",
                                for _ in 0..5 {
                                    div { class: "h-10 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                }
                            }
                        }
                    }
                } else if posts().is_empty() {
                    rsx! {
                        div { class: "text-center py-20 text-gray-500 dark:text-[#9b9c9d]",
                            "回收站为空"
                        }
                    }
                } else {
                    let list = posts();
                    let all_selected = list.iter().all(|p| selected_ids().contains(&p.id));
                    let all_ids: Vec<i32> = list.iter().map(|p| p.id).collect();
                    rsx! {
                        div { class: "bg-white dark:bg-[#2e2e33] rounded-xl border border-gray-200 dark:border-[#333] overflow-hidden",
                            div { class: "overflow-x-auto",
                                table { class: "w-full text-sm",
                                    thead {
                                        tr { class: "border-b border-gray-200 dark:border-[#333] text-left text-gray-500 dark:text-[#9b9c9d]",
                                            th { class: "px-4 py-3 font-medium w-10",
                                                input {
                                                    r#type: "checkbox",
                                                    class: "rounded border-gray-300 dark:border-[#555]",
                                                    checked: all_selected,
                                                    onchange: {
                                                        move |_| {
                                                            let mut s = selected_ids();
                                                            if all_selected {
                                                                for id in &all_ids { s.remove(id); }
                                                            } else {
                                                                for id in &all_ids { s.insert(*id); }
                                                            }
                                                            selected_ids.set(s);
                                                        }
                                                    }
                                                }
                                            }
                                            th { class: "px-4 py-3 font-medium", "标题" }
                                            th { class: "px-4 py-3 font-medium", "原状态" }
                                            th { class: "px-4 py-3 font-medium w-28", "删除时间" }
                                            th { class: "px-4 py-3 font-medium w-24 text-center", "剩余" }
                                            th { class: "px-4 py-3 font-medium w-32 text-right", "操作" }
                                        }
                                    }
                                    tbody {
                                        for post in list.iter() {
                                            TrashRow {
                                                key: "{post.id}",
                                                post: post.clone(),
                                                retention_days: settings().retention_days,
                                                selected: selected_ids().contains(&post.id),
                                                on_select: {
                                                    let id = post.id;
                                                    move |checked: bool| {
                                                        let mut s = selected_ids();
                                                        if checked { s.insert(id); } else { s.remove(&id); }
                                                        selected_ids.set(s);
                                                    }
                                                },
                                                on_restore: {
                                                    let id = post.id;
                                                    move |_| {
                                                        spawn(async move {
                                                            let _ = restore_post(id).await;
                                                        });
                                                        remove_post(id);
                                                    }
                                                },
                                                on_purge: {
                                                    let id = post.id;
                                                    move |_| {
                                                        #[cfg(target_arch = "wasm32")]
                                                        {
                                                            if web_sys::window()
                                                                .and_then(|w| w.confirm_with_message("确定要彻底删除这篇文章吗？此操作不可恢复。").ok())
                                                                .unwrap_or(false)
                                                            {
                                                                spawn(async move {
                                                                    let _ = purge_post(id).await;
                                                                });
                                                                remove_post(id);
                                                            }
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // 底部：清空回收站 + 分页
                        div { class: "flex items-center justify-between mt-4",
                            button {
                                class: "px-4 py-2 text-sm font-medium text-red-600 dark:text-red-400 border border-red-300 dark:border-red-900/50 rounded-full hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors cursor-pointer",
                                onclick: move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        if web_sys::window()
                                            .and_then(|w| w.confirm_with_message("确定要清空回收站吗？所有已删除文章将被彻底移除，此操作不可恢复。").ok())
                                            .unwrap_or(false)
                                        {
                                            spawn(async move {
                                                let _ = empty_trash().await;
                                            });
                                            posts.set(Vec::new());
                                            total.set(0);
                                            selected_ids.set(HashSet::new());
                                        }
                                    }
                                },
                                "清空回收站"
                            }
                        }
                        TrashPagination { current_page, total: total() }
                    }
                }
            }
        }
    }
}

/// 计算剩余天数（保留期 - 已删除天数）。
///
/// 返回 (剩余天数, 是否已过期)。基于客户端时钟计算，轻微漂移可接受。
fn remaining_days(post: &Post, retention_days: i32) -> (i64, bool) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(deleted_at) = post.deleted_at {
            let now_ms = js_sys::Date::now() as i64; // 毫秒
            let deleted_ms = deleted_at.timestamp_millis();
            let elapsed_days = (now_ms - deleted_ms) / 86_400_000;
            let remaining = retention_days as i64 - elapsed_days;
            (remaining, remaining <= 0)
        } else {
            (retention_days as i64, false)
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = post;
        (retention_days as i64, false)
    }
}

/// 回收站表格行组件。
#[component]
fn TrashRow(
    post: Post,
    retention_days: i32,
    selected: bool,
    on_select: EventHandler<bool>,
    on_restore: EventHandler,
    on_purge: EventHandler,
) -> Element {
    let (remaining, expired) = remaining_days(&post, retention_days);
    // 剩余天数徽章配色：>7 天中性，≤7 天鼠尾草绿(主题色)，≤0/过期琥珀色。
    let badge_class = if expired {
        "bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400"
    } else if remaining <= 7 {
        "bg-[#e8f0e8] text-[#5c7a5e] dark:bg-[#1e2e1e] dark:text-[#7da97f]"
    } else {
        "bg-gray-100 text-gray-600 dark:bg-[#333] dark:text-[#9b9c9d]"
    };
    let badge_text = if expired { "待清理".to_string() } else { format!("{remaining}天") };
    let deleted_str = post
        .deleted_at
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "—".to_string());

    rsx! {
        tr { class: "border-b border-gray-100 dark:border-[#333] last:border-0 hover:bg-gray-50 dark:hover:bg-[#2a2a2a] transition-colors",
            td { class: "px-4 py-3",
                input {
                    r#type: "checkbox",
                    class: "rounded border-gray-300 dark:border-[#555]",
                    checked: selected,
                    onchange: move |e| on_select.call(e.checked()),
                }
            }
            td { class: "px-4 py-3",
                div { class: "text-sm font-medium text-gray-900 dark:text-[#dadadb] truncate max-w-xs",
                    "{post.title}"
                }
            }
            td { class: "px-4 py-3",
                span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium whitespace-nowrap {post.status_badge_class()}",
                    "{post.status_label()}"
                }
            }
            td { class: "px-4 py-3 text-sm text-gray-500 dark:text-[#9b9c9d]",
                "{deleted_str}"
            }
            td { class: "px-4 py-3 text-center",
                span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium whitespace-nowrap {badge_class}",
                    "{badge_text}"
                }
            }
            td { class: "px-4 py-3 text-right",
                div { class: "flex justify-end gap-2",
                    button {
                        class: "text-xs text-[#5c7a5e] hover:text-[#3d5a3f] dark:text-[#7da97f] dark:hover:text-[#9dc79f] transition-colors cursor-pointer",
                        onclick: move |_| on_restore.call(()),
                        "恢复"
                    }
                    button {
                        class: "text-xs text-red-500 hover:text-red-700 dark:hover:text-red-300 transition-colors cursor-pointer",
                        onclick: move |_| on_purge.call(()),
                        "彻底删除"
                    }
                }
            }
        }
    }
}

/// 回收站分页导航组件。
#[component]
fn TrashPagination(current_page: i32, total: i64) -> Element {
    let has_prev = current_page > 1;
    let total_pages =
        ((total + TRASH_PER_PAGE as i64 - 1) / TRASH_PER_PAGE as i64).max(1) as i32;
    let has_next = current_page < total_pages;

    let prev_route = if current_page - 1 <= 1 {
        Route::Trash {}
    } else {
        Route::TrashPage { page: current_page - 1 }
    };
    let next_route = Route::TrashPage { page: current_page + 1 };

    rsx! {
        nav { class: "flex mt-6 justify-between",
            if has_prev {
                Link {
                    class: "inline-flex items-center px-4 py-2 text-sm text-white bg-gray-900 dark:bg-[#dadadb] dark:text-gray-900 rounded-full hover:opacity-80 transition-opacity cursor-pointer",
                    to: prev_route,
                    span { class: "mr-1", "«" }
                    "上一页"
                }
            } else {
                span { class: "inline-flex items-center px-4 py-2 text-sm text-gray-400 bg-gray-100 dark:bg-[#2a2a2a] rounded-full cursor-not-allowed",
                    span { class: "mr-1", "«" }
                    "上一页"
                }
            }
            span { class: "text-sm text-gray-500 dark:text-[#9b9c9d] self-center",
                "{current_page} / {total_pages} 页 (共 {total} 条)"
            }
            if has_next {
                Link {
                    class: "inline-flex items-center px-4 py-2 text-sm text-white bg-gray-900 dark:bg-[#dadadb] dark:text-gray-900 rounded-full hover:opacity-80 transition-opacity cursor-pointer",
                    to: next_route,
                    "下一页"
                    span { class: "ml-1", "»" }
                }
            } else {
                span { class: "inline-flex items-center px-4 py-2 text-sm text-gray-400 bg-gray-100 dark:bg-[#2a2a2a] rounded-full cursor-not-allowed",
                    "下一页"
                    span { class: "ml-1", "»" }
                }
            }
        }
    }
}

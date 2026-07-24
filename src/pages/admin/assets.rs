//! 素材管理页面。
//!
//! 网格浏览 `uploads/` 已登记图片：搜索（文件名/alt）、引用状态筛选
//! （全部/引用中/孤儿）、排序（最新/最大）、客户端分页。
//! 缩略图直接复用 `serve_image` 的动态处理（`?thumb=300x300`），零额外成本。

use dioxus::prelude::*;

use crate::api::assets::{delete_asset, list_assets, purge_orphan_assets, update_asset_alt};
use crate::api::assets::{AssetListResponse, PurgeOrphansResponse};
use crate::components::empty_state::EmptyState;
use crate::components::ui::{FilterTabs, Pagination};
use crate::models::asset::{AssetFilter, AssetSort};

/// 每页素材数，与服务端 list.rs 的 PER_PAGE 对齐。
const ASSETS_PER_PAGE: i32 = 60;

/// 格式化字节数为可读字符串（B/KB/MB/GB）。
fn format_bytes(bytes: i64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// 素材管理入口组件。
#[component]
pub fn Assets() -> Element {
    // 筛选/搜索/排序/分页状态：全部客户端驱动（单路由 + signal，对齐「管理文章」模式）。
    let mut filter = use_signal(|| "all".to_string());
    let mut query = use_signal(String::new);
    let mut sort = use_signal(|| "created".to_string());
    let mut page = use_signal(|| 1_i32);

    let mut data: Signal<Option<AssetListResponse>> = use_signal(|| None);
    #[allow(unused_mut)]
    let mut loading: Signal<bool> = use_signal(|| true);
    #[allow(unused_mut)]
    let mut error: Signal<Option<String>> = use_signal(|| None);

    // 操作结果横幅（删除/清理/alt 编辑的反馈）。
    #[allow(unused_mut)]
    let mut op_message: Signal<Option<String>> = use_signal(|| None);
    // 待二次确认的删除目标（素材 id）与一键清理确认态。
    let mut confirm_delete: Signal<Option<String>> = use_signal(|| None);
    let mut purge_confirm = use_signal(|| false);
    // alt 内联编辑：目标素材 id + 输入框值。
    let mut editing_alt: Signal<Option<String>> = use_signal(|| None);
    let mut alt_input = use_signal(String::new);
    // 重载触发器：操作成功后 +1 让 effect 重新请求。
    let mut reload = use_signal(|| 0_i32);

    // 数据加载：任一查询条件或 reload 变化时重新请求。筛选/搜索/排序变化时重置到第 1 页。
    use_effect(move || {
        let f = filter();
        let q = query();
        let s = sort();
        let p = page();
        let _ = reload();

        #[cfg(target_arch = "wasm32")]
        {
            let filter_enum = match f.as_str() {
                "used" => AssetFilter::Used,
                "orphan" => AssetFilter::Orphan,
                _ => AssetFilter::All,
            };
            let sort_enum = if s == "size" {
                AssetSort::SizeDesc
            } else {
                AssetSort::CreatedDesc
            };
            spawn(async move {
                loading.set(true);
                error.set(None);
                match list_assets(filter_enum, q, sort_enum, p).await {
                    Ok(resp) => data.set(Some(resp)),
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
        }
    });

    let resp = data.read();
    let (assets, total, used_count, orphan_count, purgeable_count, purgeable_bytes) =
        match resp.as_ref() {
            Some(r) => (
                r.assets.clone(),
                r.total,
                r.used_count,
                r.orphan_count,
                r.purgeable_count,
                r.purgeable_bytes,
            ),
            None => (Vec::new(), 0, 0, 0, 0, 0),
        };
    let all_count = used_count + orphan_count;
    drop(resp);

    // Dioxus 格式化段不支持内联 if 块表达式，条件 class 提前算好。
    let sort_btn_base = "text-xs font-mono tracking-widest uppercase cursor-pointer px-3 py-2 rounded-full border transition-colors";
    let sort_active = "border-[var(--color-paper-primary)] text-[var(--color-paper-primary)]";
    let sort_idle = "border-[var(--color-paper-border)] text-[var(--color-paper-secondary)] hover:text-[var(--color-paper-primary)]";
    let sort_created_class = if sort() == "created" {
        format!("{sort_btn_base} {sort_active}")
    } else {
        format!("{sort_btn_base} {sort_idle}")
    };
    let sort_size_class = if sort() == "size" {
        format!("{sort_btn_base} {sort_active}")
    } else {
        format!("{sort_btn_base} {sort_idle}")
    };

    rsx! {
        div {
            h1 { class: "text-3xl font-extrabold tracking-tight mb-2", "素材管理" }
            p { class: "text-sm text-[var(--color-paper-secondary)] mb-8",
                "管理文章编辑器上传的图片。共 {all_count} 张，引用中 {used_count} 张，孤儿 {orphan_count} 张。"
            }

            // 顶栏：筛选 tabs + 搜索 + 排序
            div { class: "flex flex-wrap items-end justify-between gap-4",
                FilterTabs {
                    items: vec![("all", "全部"), ("used", "引用中"), ("orphan", "孤儿")],
                    active_value: filter(),
                    on_change: move |v: String| {
                        filter.set(v);
                        page.set(1);
                    },
                }
                div { class: "flex items-center gap-3 pb-1",
                    input {
                        class: "w-56 text-sm bg-[var(--color-paper-entry)] text-[var(--color-paper-primary)] placeholder-[var(--color-paper-tertiary)] focus:outline-none border border-[var(--color-paper-border)] focus:border-[var(--color-paper-primary)] rounded-2xl px-4 py-2 shadow-sm transition-all",
                        r#type: "search",
                        placeholder: "搜索文件名 / alt",
                        value: "{query}",
                        oninput: move |evt| {
                            query.set(evt.value());
                            page.set(1);
                        },
                    }
                    button {
                        class: "{sort_created_class}",
                        onclick: move |_| {
                            sort.set("created".to_string());
                            page.set(1);
                        },
                        "最新"
                    }
                    button {
                        class: "{sort_size_class}",
                        onclick: move |_| {
                            sort.set("size".to_string());
                            page.set(1);
                        },
                        "最大"
                    }
                    // 一键清理孤儿：仅 7 天保护窗外的无引用素材；两步确认。
                    if purgeable_count > 0 {
                        if purge_confirm() {
                            button {
                                class: "text-xs font-medium cursor-pointer px-3 py-2 rounded-full bg-red-500 text-white hover:bg-red-600 transition-colors",
                                onclick: move |_| {
                                    purge_confirm.set(false);
                                    #[cfg(target_arch = "wasm32")]
                                    spawn(async move {
                                        match purge_orphan_assets().await {
                                            Ok(PurgeOrphansResponse {
                                                deleted_count,
                                                freed_bytes,
                                                failures,
                                                ..
                                            }) => {
                                                let mut msg = format!(
                                                    "已清理 {} 张孤儿素材，释放 {}",
                                                    deleted_count,
                                                    format_bytes(freed_bytes)
                                                );
                                                if failures > 0 {
                                                    msg.push_str(&format!(
                                                        "（{} 个文件删除失败）",
                                                        failures
                                                    ));
                                                }
                                                op_message.set(Some(msg));
                                                reload.set(reload() + 1);
                                            }
                                            Err(e) => {
                                                op_message.set(Some(format!("清理失败：{e}")))
                                            }
                                        }
                                    });
                                },
                                "确认清理 {purgeable_count} 张（{format_bytes(purgeable_bytes)}）"
                            }
                            button {
                                class: "text-xs cursor-pointer px-3 py-2 rounded-full border border-[var(--color-paper-border)] text-[var(--color-paper-secondary)] hover:text-[var(--color-paper-primary)] transition-colors",
                                onclick: move |_| purge_confirm.set(false),
                                "取消"
                            }
                        } else {
                            button {
                                class: "text-xs font-medium cursor-pointer px-3 py-2 rounded-full border border-amber-500/50 text-amber-600 dark:text-amber-400 hover:bg-amber-500/10 transition-colors",
                                title: "仅清理无引用且上传超过 7 天的素材（保护未保存的草稿）",
                                onclick: move |_| purge_confirm.set(true),
                                "清理孤儿（{purgeable_count} 张 · {format_bytes(purgeable_bytes)}）"
                            }
                        }
                    }
                }
            }

            // 操作结果横幅
            if let Some(msg) = op_message() {
                div { class: "mt-4 flex items-center justify-between gap-4 rounded-2xl border border-[var(--color-paper-border)] bg-[var(--color-paper-entry)] px-4 py-3 text-sm text-[var(--color-paper-primary)] shadow-sm",
                    span { "{msg}" }
                    button {
                        class: "text-[var(--color-paper-tertiary)] hover:text-[var(--color-paper-primary)] cursor-pointer",
                        onclick: move |_| op_message.set(None),
                        "×"
                    }
                }
            }

            // 内容区
            if let Some(err) = error() {
                div { class: "mt-8 text-sm text-red-500", "加载失败：{err}" }
            } else if loading() && assets.is_empty() {
                div { class: "mt-8 text-sm text-[var(--color-paper-secondary)]", "加载中..." }
            } else if assets.is_empty() {
                EmptyState {
                    title: "暂无素材".to_string(),
                    description: "在编辑器中上传图片后会自动出现在这里".to_string(),
                }
            } else {
                // 网格：缩略图卡片
                div { class: "grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 gap-4 mt-2",
                    for asset in assets.iter() {
                        {
                            let a = &asset.asset;
                            let thumb = format!("/uploads/{}?thumb=300x300", a.path);
                            let is_orphan = asset.ref_count == 0;
                            let badge_class = if is_orphan {
                                "absolute top-2 left-2 text-[10px] font-mono px-2 py-0.5 rounded-full backdrop-blur-sm bg-amber-500/80 text-white"
                            } else {
                                "absolute top-2 left-2 text-[10px] font-mono px-2 py-0.5 rounded-full backdrop-blur-sm bg-black/50 text-white"
                            };
                            rsx! {
                                div {
                                    key: "{a.id}",
                                    class: "group relative rounded-3xl overflow-hidden border border-[var(--color-paper-border)] bg-[var(--color-paper-entry)] shadow-sm hover:shadow-md transition-all",
                                    div { class: "aspect-square overflow-hidden bg-[var(--color-paper-theme)]",
                                        img {
                                            class: "w-full h-full object-cover",
                                            src: "{thumb}",
                                            alt: a.alt.clone().unwrap_or_else(|| a.filename.clone()),
                                            loading: "lazy",
                                        }
                                    }
                                    // 引用徽标
                                    span {
                                        class: "{badge_class}",
                                        if is_orphan {
                                            "孤儿"
                                        } else {
                                            "被 {asset.ref_count} 篇引用"
                                        }
                                    }
                                    div { class: "p-3",
                                        p { class: "text-xs font-medium truncate text-[var(--color-paper-primary)]",
                                            title: "{a.filename}",
                                            "{a.filename}"
                                        }
                                        p { class: "text-[10px] font-mono text-[var(--color-paper-tertiary)] mt-0.5",
                                            "{a.width}×{a.height} · {format_bytes(a.size_bytes)}"
                                        }
                                        if let Some(alt_text) = &a.alt {
                                            p { class: "text-[10px] truncate text-[var(--color-paper-secondary)] mt-0.5",
                                                title: "{alt_text}",
                                                "alt: {alt_text}"
                                            }
                                        }

                                        // 操作区：确认删除 / alt 编辑 / 常规三按钮 三态互斥
                                        if confirm_delete().as_deref() == Some(a.id.as_str()) {
                                            div { class: "flex items-center gap-2 mt-2",
                                                button {
                                                    class: "text-[10px] font-medium cursor-pointer px-2 py-1 rounded-full bg-red-500 text-white hover:bg-red-600 transition-colors",
                                                    onclick: {
                                                        let id = a.id.clone();
                                                        move |_| {
                                                            confirm_delete.set(None);
                                                            let id = id.clone();
                                                            #[cfg(target_arch = "wasm32")]
                                                            spawn(async move {
                                                                match delete_asset(id).await {
                                                                    Ok(resp) => {
                                                                        op_message.set(Some(resp.message));
                                                                        if resp.success {
                                                                            reload.set(reload() + 1);
                                                                        }
                                                                    }
                                                                    Err(e) => op_message
                                                                        .set(Some(format!("删除失败：{e}"))),
                                                                }
                                                            });
                                                        }
                                                    },
                                                    "确认删除"
                                                }
                                                button {
                                                    class: "text-[10px] cursor-pointer px-2 py-1 rounded-full border border-[var(--color-paper-border)] text-[var(--color-paper-secondary)] hover:text-[var(--color-paper-primary)] transition-colors",
                                                    onclick: move |_| confirm_delete.set(None),
                                                    "取消"
                                                }
                                            }
                                        } else if editing_alt().as_deref() == Some(a.id.as_str()) {
                                            div { class: "flex items-center gap-1 mt-2",
                                                input {
                                                    class: "flex-1 min-w-0 text-[10px] bg-[var(--color-paper-entry)] text-[var(--color-paper-primary)] focus:outline-none border border-[var(--color-paper-border)] focus:border-[var(--color-paper-primary)] rounded-full px-2 py-1 transition-all",
                                                    r#type: "text",
                                                    placeholder: "alt 文本",
                                                    value: "{alt_input}",
                                                    oninput: move |evt| alt_input.set(evt.value()),
                                                }
                                                button {
                                                    class: "text-[10px] font-medium cursor-pointer px-2 py-1 rounded-full bg-[var(--color-paper-primary)] text-[var(--color-paper-bg)] hover:opacity-80 transition-opacity",
                                                    onclick: {
                                                        let id = a.id.clone();
                                                        move |_| {
                                                            let id = id.clone();
                                                            let alt = alt_input();
                                                            editing_alt.set(None);
                                                            #[cfg(target_arch = "wasm32")]
                                                            spawn(async move {
                                                                match update_asset_alt(id, alt).await {
                                                                    Ok(resp) => {
                                                                        op_message.set(Some(resp.message));
                                                                        if resp.success {
                                                                            reload.set(reload() + 1);
                                                                        }
                                                                    }
                                                                    Err(e) => op_message
                                                                        .set(Some(format!("保存失败：{e}"))),
                                                                }
                                                            });
                                                        }
                                                    },
                                                    "存"
                                                }
                                                button {
                                                    class: "text-[10px] cursor-pointer px-2 py-1 rounded-full border border-[var(--color-paper-border)] text-[var(--color-paper-secondary)] hover:text-[var(--color-paper-primary)] transition-colors",
                                                    onclick: move |_| editing_alt.set(None),
                                                    "×"
                                                }
                                            }
                                        } else {
                                            div { class: "flex items-center gap-2 mt-2 opacity-0 group-hover:opacity-100 transition-opacity",
                                                button {
                                                    class: "text-[10px] cursor-pointer text-[var(--color-paper-secondary)] hover:text-[var(--color-paper-primary)] transition-colors",
                                                    title: "复制图片 URL",
                                                    onclick: {
                                                        let url = format!("/uploads/{}", a.path);
                                                        move |_| {
                                                            #[cfg(target_arch = "wasm32")]
                                                            if let Some(window) = web_sys::window() {
                                                                let _ = window
                                                                    .navigator()
                                                                    .clipboard()
                                                                    .write_text(&url);
                                                                op_message.set(Some(format!("已复制 {url}")));
                                                            }
                                                        }
                                                    },
                                                    "复制"
                                                }
                                                button {
                                                    class: "text-[10px] cursor-pointer text-[var(--color-paper-secondary)] hover:text-[var(--color-paper-primary)] transition-colors",
                                                    title: "编辑 alt",
                                                    onclick: {
                                                        let id = a.id.clone();
                                                        let current_alt = a.alt.clone().unwrap_or_default();
                                                        move |_| {
                                                            alt_input.set(current_alt.clone());
                                                            editing_alt.set(Some(id.clone()));
                                                        }
                                                    },
                                                    "alt"
                                                }
                                                if asset.ref_count > 0 {
                                                    {
                                                        let refs_tip = asset
                                                            .refs
                                                            .iter()
                                                            .map(|r| r.title.clone())
                                                            .collect::<Vec<_>>()
                                                            .join("、");
                                                        rsx! {
                                                            span {
                                                                class: "text-[10px] text-[var(--color-paper-tertiary)] cursor-not-allowed",
                                                                title: "被引用：{refs_tip}",
                                                                "删除"
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    button {
                                                        class: "text-[10px] cursor-pointer text-red-500/70 hover:text-red-500 transition-colors",
                                                        onclick: {
                                                            let id = a.id.clone();
                                                            move |_| confirm_delete.set(Some(id.clone()))
                                                        },
                                                        "删除"
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

                Pagination {
                    variant: "admin",
                    current_page: page(),
                    total,
                    per_page: ASSETS_PER_PAGE,
                    unit: "张",
                    on_prev: move |_| page.set((page() - 1).max(1)),
                    on_next: move |_| page.set(page() + 1),
                }
            }
        }
    }
}

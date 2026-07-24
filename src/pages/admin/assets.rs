//! 素材管理页面。
//!
//! 网格浏览 `uploads/` 已登记图片：搜索（文件名/alt）、引用状态筛选
//! （全部/引用中/孤儿）、排序（最新/最大）、客户端分页。
//! 缩略图直接复用 `serve_image` 的动态处理（`?thumb=300x300`），零额外成本。

use dioxus::prelude::*;

use crate::api::assets::list_assets;
use crate::api::assets::AssetListResponse;
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

    // 数据加载：任一查询条件变化时重新请求。筛选/搜索/排序变化时重置到第 1 页。
    use_effect(move || {
        let f = filter();
        let q = query();
        let s = sort();
        let p = page();

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
    let (assets, total, used_count, orphan_count) = match resp.as_ref() {
        Some(r) => (
            r.assets.clone(),
            r.total,
            r.used_count,
            r.orphan_count,
        ),
        None => (Vec::new(), 0, 0, 0),
    };
    let all_count = used_count + orphan_count;

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

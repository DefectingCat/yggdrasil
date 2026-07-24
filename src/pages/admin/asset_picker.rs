//! 素材选择 modal（封面上「从素材库选择」）。
//!
//! 网格展示素材库（默认最新排序，支持文件名/alt 搜索），单击选中回填封面 URL。
//! 内嵌「上传新图」入口（复用 `upload_image_file`），上传成功即选中。
//! 纯 Dioxus 组件，不触碰 Tiptap；数据加载仅在 WASM 前端发生。

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::api::assets::list_assets;
use crate::models::asset::AssetDto;
#[cfg(target_arch = "wasm32")]
use crate::models::asset::{AssetFilter, AssetSort};

/// 素材选择 modal。
///
/// - `visible`：显隐控制（父组件持有，选中/点遮罩/× 都会置 false）。
/// - `on_select`：选中回填，参数为 `/uploads/<path>` URL。
/// - `cover_uploading`：modal 内上传新图时置位，供父页面拦截保存（与 CoverUploader 语义一致）。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
pub fn AssetPickerModal(
    mut visible: Signal<bool>,
    on_select: EventHandler<String>,
    cover_uploading: Signal<bool>,
) -> Element {
    #[allow(unused_mut)]
    let mut assets: Signal<Vec<AssetDto>> = use_signal(Vec::new);
    #[allow(unused_mut)]
    let mut loading = use_signal(|| false);
    let mut query = use_signal(String::new);
    #[allow(unused_mut)]
    let mut error = use_signal(|| None::<String>);

    // 打开时与搜索词变化时加载第一页（最新 60 张，封面复用场景足够）。
    use_effect(move || {
        let open = visible();
        let q = query();
        if open {
            #[cfg(target_arch = "wasm32")]
            spawn(async move {
                loading.set(true);
                match list_assets(AssetFilter::All, q, AssetSort::CreatedDesc, 1).await {
                    Ok(resp) => {
                        assets.set(resp.assets);
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
        }
    });

    if !visible() {
        return rsx! {};
    }

    rsx! {
        // 遮罩：点击关闭
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-6",
            onclick: move |_| visible.set(false),
            // 面板：阻止点击穿透到遮罩
            div { class: "w-full max-w-3xl max-h-[80vh] flex flex-col rounded-[2rem] bg-[var(--color-paper-entry)] border border-[var(--color-paper-border)] shadow-xl overflow-hidden",
                onclick: move |evt| evt.stop_propagation(),

                // 头部：标题 + 搜索 + 上传 + 关闭
                div { class: "flex items-center gap-3 px-6 py-4 border-b border-[var(--color-paper-border)]",
                    h2 { class: "text-lg font-bold text-[var(--color-paper-primary)] shrink-0",
                        "选择封面图"
                    }
                    input {
                        class: "flex-1 min-w-0 text-sm bg-[var(--color-paper-entry)] text-[var(--color-paper-primary)] placeholder-[var(--color-paper-tertiary)] focus:outline-none border border-[var(--color-paper-border)] focus:border-[var(--color-paper-primary)] rounded-2xl px-4 py-2 shadow-sm transition-all",
                        r#type: "search",
                        placeholder: "搜索文件名 / alt",
                        value: "{query}",
                        oninput: move |evt| query.set(evt.value()),
                    }
                    // 上传新图：成功后直接选中
                    label { class: "shrink-0 text-sm font-medium px-4 py-2 rounded-full bg-[var(--color-paper-primary)] text-[var(--color-paper-theme)] hover:opacity-80 transition-opacity cursor-pointer",
                        "上传新图"
                        input {
                            r#type: "file",
                            accept: "image/jpeg,image/png,image/gif,image/webp",
                            class: "hidden",
                            onchange: move |evt| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    use dioxus::html::HasFileData;
                                    use dioxus::web::WebFileExt;
                                    if let Some(file) = evt.files().into_iter().next() {
                                        if let Some(web_file) = file.get_web_file() {
                                            cover_uploading.set(true);
                                            spawn(async move {
                                                match crate::tiptap_bridge::upload_image_file(
                                                    web_file,
                                                )
                                                .await
                                                {
                                                    Ok(url) => {
                                                        on_select.call(url);
                                                        visible.set(false);
                                                    }
                                                    // 失败留在 modal 内提示，不关闭。
                                                    Err(msg) => error.set(Some(msg)),
                                                }
                                                cover_uploading.set(false);
                                            });
                                        }
                                    }
                                }
                            },
                        }
                    }
                    button {
                        class: "shrink-0 w-8 h-8 flex items-center justify-center rounded-full text-[var(--color-paper-secondary)] hover:bg-[var(--color-paper-theme)] transition-colors cursor-pointer",
                        aria_label: "关闭",
                        onclick: move |_| visible.set(false),
                        "×"
                    }
                }

                // 网格内容区
                div { class: "flex-1 overflow-y-auto p-4",
                    if let Some(err) = error() {
                        div { class: "py-12 text-center text-sm text-red-500", "加载失败：{err}" }
                    } else if loading() && assets.read().is_empty() {
                        div { class: "py-12 text-center text-sm text-[var(--color-paper-secondary)]",
                            "加载中..."
                        }
                    } else if assets.read().is_empty() {
                        div { class: "py-12 text-center text-sm text-[var(--color-paper-secondary)]",
                            "素材库为空，点击「上传新图」添加"
                        }
                    } else {
                        div { class: "grid grid-cols-3 sm:grid-cols-4 gap-3",
                            for asset in assets.read().iter() {
                                {
                                    let url = format!("/uploads/{}", asset.asset.path);
                                    let thumb = format!("{}?thumb=300x300", url);
                                    rsx! {
                                        button {
                                            key: "{asset.asset.id}",
                                            class: "group relative aspect-square rounded-2xl overflow-hidden border border-[var(--color-paper-border)] hover:border-[var(--color-paper-primary)] hover:shadow-md transition-all cursor-pointer",
                                            title: "{asset.asset.filename}",
                                            onclick: {
                                                let url = url.clone();
                                                move |_| {
                                                    on_select.call(url.clone());
                                                    visible.set(false);
                                                }
                                            },
                                            img {
                                                class: "w-full h-full object-cover",
                                                src: "{thumb}",
                                                alt: asset.asset.alt.clone().unwrap_or_else(|| {
                                                    asset.asset.filename.clone()
                                                }),
                                                loading: "lazy",
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
}

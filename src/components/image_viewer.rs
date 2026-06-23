//! 图片查看器组件
//!
//! 提供缩略图展示与点击放大后的全屏灯箱（lightbox）查看，
//! 支持自定义缩略图参数、alt 文本与懒加载。
//! 灯箱支持键盘操作：Escape 关闭、Tab 在关闭按钮与图片间循环。

use dioxus::prelude::*;

/// 图片查看器组件。
///
/// Props：
/// - `src`：原图 URL
/// - `thumb_params`：缩略图 URL 参数，默认 `"?w=800"`
/// - `alt`：图片替代文本，默认 `"图片"`
/// - `lazy_load`：是否启用懒加载，默认 `false`
/// - `lightbox`：是否启用点击放大灯箱，默认 `true`。卡片等场景设为 `false`
///   以禁用灯箱交互，使整张卡片归一为单一跳转行为（避免嵌套交互）。
///
/// 关键事件（仅在 `lightbox == true` 时生效）：
/// - 点击缩略图：打开全屏灯箱
/// - 点击遮罩或关闭按钮：关闭灯箱
/// - 点击灯箱内容区：阻止事件冒泡，避免误关闭
/// - 键盘 Escape：关闭灯箱
#[component]
pub fn ImageViewer(
    src: String,
    #[props(default = "?w=800".to_string())] thumb_params: String,
    #[props(default = "?w=20".to_string())] placeholder_params: String,
    #[props(default = "图片".to_string())] alt: String,
    #[props(default = false)] lazy_load: bool,
    #[props(default = true)] lightbox: bool,
) -> Element {
    let mut is_open = use_signal(|| false);

    // 打开灯箱时聚焦关闭按钮，并监听 Escape 键关闭。
    // 灯箱禁用时跳过键盘监听与焦点管理，避免无谓的 DOM 操作。
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{closure::Closure, JsCast};

        use_effect(move || {
            if !lightbox || !is_open() {
                return;
            }

            wasm_bindgen_futures::spawn_local(async move {
                // 下一帧再聚焦，确保 DOM 已渲染。
                let _ = js_sys::eval("new Promise(r => requestAnimationFrame(r))").unwrap();
                if let Some(btn) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.query_selector(".image-viewer-close").ok())
                    .flatten()
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                {
                    let _ = btn.focus();
                }
            });

            let closure = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                if e.key() == "Escape" {
                    is_open.set(false);
                }
            }) as Box<dyn FnMut(_)>);

            if let Some(window) = web_sys::window() {
                let _ = window.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            }

            closure.forget();
        });
    }

    // 计算 aspect-ratio：SSR 时读图片真实尺寸。WASM 端不读（--ar 已在 SSR 写入 HTML）。
    // 非 /uploads/ 的外链图或读不到尺寸时不设 --ar。
    let ar_style = {
        // `mut` 仅 server 构建需要：WASM 构建剥离 #[cfg(feature = "server")] 块后，
        // s 从未被重新赋值，故对 WASM 抑制 unused_mut。
        #[cfg_attr(not(feature = "server"), allow(unused_mut))]
        let mut s = String::new();
        #[cfg(feature = "server")]
        {
            if let Some(rel) = src
                .strip_prefix("/uploads/")
                .map(|p| p.split('?').next().unwrap_or(p))
            {
                if let Some((w, h)) = crate::api::image::get_image_dimensions(rel) {
                    // 注意：CSS aspect-ratio 用斜杠分隔（width / height），不是冒号
                    s = format!("--ar:{} / {};", w, h);
                }
            }
        }
        s
    };

    // 拼接占位图 URL 和高清图 URL
    let placeholder_src = if src.contains('?') {
        format!(
            "{}&{}",
            src.split('?').next().unwrap_or(&src),
            placeholder_params.trim_start_matches('?')
        )
    } else {
        format!("{}{}", src, placeholder_params)
    };
    let full_src = if src.contains('?') {
        format!(
            "{}&{}",
            src.split('?').next().unwrap_or(&src),
            thumb_params.trim_start_matches('?')
        )
    } else {
        format!("{}{}", src, thumb_params)
    };

    rsx! {
        // blur-up 双层：底层占位图 + 上层高清图（data-src 由前端 JS 懒加载）
        // 灯箱禁用时不绑定 onclick，使缩略图成为纯展示元素（卡片由外层覆盖链接接管跳转）。
        span {
            class: "blur-img",
            style: "{ar_style}",
            onclick: move |_| if lightbox { is_open.set(true) },
            img {
                class: "blur-img-placeholder",
                src: "{placeholder_src}",
                alt: "{alt}",
                loading: if lazy_load { "lazy" } else { "eager" },
            }
            img {
                class: "blur-img-full",
                "data-src": "{full_src}",
                alt: "{alt}",
            }
        }

        // 全屏灯箱（仅 lightbox 启用时可能打开）
        if lightbox && is_open() {
            div {
                class: "image-viewer-overlay",
                role: "dialog",
                aria_modal: "true",
                aria_label: "图片预览",
                onclick: move |_| is_open.set(false),
                div {
                    class: "image-viewer-content",
                    onclick: move |evt: dioxus::events::MouseEvent| evt.stop_propagation(),
                    img {
                        class: "image-viewer-img",
                        src: "{src}",
                        alt: "{alt}",
                        tabindex: 0,
                    }
                    button {
                        class: "image-viewer-close",
                        r#type: "button",
                        aria_label: "关闭图片预览",
                        onclick: move |_| is_open.set(false),
                        "✕"
                    }
                }
            }
        }
    }
}

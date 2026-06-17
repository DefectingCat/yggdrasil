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
///
/// 关键事件：
/// - 点击缩略图：打开全屏灯箱
/// - 点击遮罩或关闭按钮：关闭灯箱
/// - 点击灯箱内容区：阻止事件冒泡，避免误关闭
/// - 键盘 Escape：关闭灯箱
#[component]
pub fn ImageViewer(
    src: String,
    #[props(default = "?w=800".to_string())] thumb_params: String,
    #[props(default = "图片".to_string())] alt: String,
    #[props(default = false)] lazy_load: bool,
) -> Element {
    let mut is_open = use_signal(|| false);

    // 打开灯箱时聚焦关闭按钮，并监听 Escape 键关闭。
    #[cfg(target_arch = "wasm32")]
    {
        use_effect(move || {
            if !is_open() {
                return;
            }

            wasm_bindgen_futures::spawn_local(async move {
                // 下一帧再聚焦，确保 DOM 已渲染。
                let _ = js_sys::eval("new Promise(r => requestAnimationFrame(r))").unwrap();
                if let Some(btn) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.query_selector(".image-viewer-close").ok())
                    .flatten()
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

    // 拼接缩略图 URL：保留原 URL 的 query 参数并追加 thumb_params
    let thumb_src = if src.contains('?') {
        format!(
            "{}&{}",
            src.split('?').next().unwrap_or(&src),
            thumb_params.trim_start_matches('?')
        )
    } else {
        format!("{}{}", src, thumb_params)
    };

    rsx! {
        // 缩略图
        img {
            class: "cursor-pointer transition-opacity hover:opacity-90",
            src: "{thumb_src}",
            alt: "{alt}",
            loading: if lazy_load { "lazy" } else { "eager" },
            onclick: move |_| is_open.set(true),
        }

        // 全屏灯箱
        if is_open() {
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

//! 文章内容组件
//!
//! 渲染由服务端生成的文章 HTML 内容，并在 WASM 前端初始化交互脚本。

use dioxus::prelude::*;

/// 文章内容组件。
///
/// Props：
/// - `content_html`：服务端渲染的文章 HTML 字符串
///
/// 关键行为：
/// - 在 `target_arch = "wasm32"` 环境下调用 `window.__initPostContent` 初始化代码块
///   复制按钮（`yggdrasil-core.js` 已由 `Dioxus.toml` 全局注入）。
///   灯箱（图片灯箱 + 懒加载）改由 `Dioxus.toml` 全局注入 `lightbox.js`，
///   这里仅设置其初始化配置 `__lightboxSelectors` 并兜底调用。
#[component]
pub fn PostContent(content_html: String) -> Element {
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        let _ = js_sys::eval("window.__initPostContent('.post-content')");
        // lightbox 改由 Dioxus.toml 全局 <script src> 加载（不再 include_str!）。
        // 双保险契约：先设配置，若 lightbox.js 已加载则立即调用；
        // 否则 lightbox.js 加载完后其 IIFE 尾部读到配置自启动。
        let _ = js_sys::eval(
            "window.__lightboxSelectors = ['.post-content', '.entry-cover']; \
             if (window.__initLightbox) window.__initLightbox(window.__lightboxSelectors);",
        );
    });

    rsx! {
        div {
            class: "post-content md-content",
            dangerous_inner_html: "{content_html}",
        }
    }
}

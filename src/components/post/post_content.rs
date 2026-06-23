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
/// - 在 `target_arch = "wasm32"` 环境下执行 `post-content.js`（代码块复制）与
///   `lightbox.js`（图片灯箱 + 懒加载），并调用各自的初始化函数。
#[component]
pub fn PostContent(content_html: String) -> Element {
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        let _ = js_sys::eval(include_str!("../../../public/js/post-content.js"));
        let _ = js_sys::eval(include_str!("../../../public/js/lightbox.js"));
        let _ = js_sys::eval("window.__initPostContent('.post-content')");
        // 正文图组成图集；封面（.entry-cover）单张模式，由 PostCover 标记 data-single。
        let _ = js_sys::eval("window.__initLightbox(['.post-content', '.entry-cover'])");
    });

    rsx! {
        div {
            class: "post-content md-content",
            dangerous_inner_html: "{content_html}"
        }
    }
}

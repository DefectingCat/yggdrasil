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
/// - 在 `target_arch = "wasm32"` 环境下执行 `post-content.js` 并调用初始化函数，
///   用于处理代码块、图片点击等前端交互
#[component]
pub fn PostContent(content_html: String) -> Element {
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        let _ = js_sys::eval(include_str!("../../../public/js/post-content.js"));
        let _ = js_sys::eval("window.__initPostContent('.post-content')");
    });

    rsx! {
        div {
            class: "post-content md-content",
            dangerous_inner_html: "{content_html}"
        }
    }
}

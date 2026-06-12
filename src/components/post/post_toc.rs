//! 文章目录组件
//!
//! 在文章详情页渲染由服务端生成的目录 HTML，支持折叠展开。

use dioxus::prelude::*;

/// 文章目录（Table of Contents）组件。
///
/// Props：
/// - `toc_html`：服务端生成的目录 HTML 字符串
///
/// 通过 `dangerous_inner_html` 注入目录结构，快捷键 `Alt + C` 可聚焦。
#[component]
pub fn PostToc(toc_html: String) -> Element {
    rsx! {
        details { class: "toc",
            summary {
                accesskey: "c",
                title: "(Alt + C)",
                span { class: "title", "Table of Contents" }
            }
            div {
                class: "inner",
                dangerous_inner_html: "{toc_html}"
            }
        }
    }
}

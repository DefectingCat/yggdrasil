//! 文章元信息组件
//!
//! 展示文章发布日期、阅读时长与字数统计。

use dioxus::prelude::*;

use crate::models::post::Post;

/// 文章元信息组件。
///
/// Props：
/// - `post`：文章数据模型
///
/// 渲染格式：`日期 · min read · words`
#[component]
pub fn PostMeta(post: Post) -> Element {
    rsx! {
        div { class: "post-meta",
            span { "{post.formatted_date()}" }
            span { "·" }
            span { "{post.reading_time} min read" }
            span { "·" }
            span { "{post.word_count} words" }
        }
    }
}

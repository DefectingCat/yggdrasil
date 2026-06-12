//! 文章上一篇/下一篇导航组件
//!
//! 在文章详情页底部提供相邻文章的快速跳转。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::models::post::PostNav;
use crate::router::Route;

/// 文章相邻导航组件。
///
/// Props：
/// - `prev`：上一篇文章摘要
/// - `next`：下一篇文章摘要
///
/// 左右两侧分别渲染 Prev/Next 链接，若无相邻文章则占位空白。
#[component]
pub fn PostNavLinks(prev: Option<PostNav>, next: Option<PostNav>) -> Element {
    rsx! {
        nav { class: "paginav",
            if let Some(prev_post) = prev {
                Link {
                    class: "prev",
                    to: Route::PostDetail { slug: prev_post.slug.clone() },
                    onclick: move |_evt: dioxus::events::MouseEvent| {},
                    span { class: "title", "« Prev" }
                    span { class: "post-title-nav", "{prev_post.title}" }
                }
            } else {
                span { class: "prev" }
            }

            if let Some(next_post) = next {
                Link {
                    class: "next",
                    to: Route::PostDetail { slug: next_post.slug.clone() },
                    onclick: move |_evt: dioxus::events::MouseEvent| {},
                    span { class: "title", "Next »" }
                    span { class: "post-title-nav", "{next_post.title}" }
                }
            } else {
                span { class: "next" }
            }
        }
    }
}

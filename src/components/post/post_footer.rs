//! 文章页脚组件
//!
//! 展示文章标签、上一篇/下一篇导航与返回首页链接。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::components::post::post_nav_links::PostNavLinks;
use crate::models::post::Post;
use crate::router::Route;

/// 文章页脚组件。
///
/// Props：
/// - `post`：文章数据模型
///
/// 展示内容包括：
/// - 文章标签云，链接到对应标签详情页
/// - 相邻文章导航（如有）
/// - 返回首页链接
#[component]
pub fn PostFooter(post: Post) -> Element {
    let tags = post.tags.clone();

    rsx! {
        footer { class: "post-footer",
            if !tags.is_empty() {
                ul { class: "post-tags",
                    for tag in tags.into_iter() {
                        li { key: "{tag}",
                            Link {
                                to: Route::TagDetail {
                                    tag: tag.clone(),
                                },
                                "{tag}"
                            }
                        }
                    }
                }
            }

            if post.prev_post.is_some() || post.next_post.is_some() {
                PostNavLinks { prev: post.prev_post, next: post.next_post }
            }

            div { class: "back-to-home",
                Link { to: Route::Home {}, "← 返回首页" }
            }
        }
    }
}

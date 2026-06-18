//! 文章卡片组件
//!
//! 在首页、标签详情等列表中展示单篇文章的标题、摘要、封面、日期与标签。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::components::image_viewer::ImageViewer;
use crate::models::post::PostListItem;
use crate::router::Route;

/// 文章卡片组件。
///
/// Props：
/// - `post`：文章数据模型
///
/// 展示内容包括：
/// - 封面图（如有，使用 400x300 缩略图）
/// - 文章标题
/// - 摘要（最多两行）
/// - 发布日期与标签
///
/// 关键事件：
/// - 点击标签时阻止事件冒泡，避免触发整卡跳转
#[component]
pub fn PostCard(post: PostListItem) -> Element {
    let post_slug = post.slug.clone();
    let date_str = post.formatted_date();
    let has_cover = post.cover_image.is_some();

    rsx! {
        article {
            class: "relative mb-6 p-6 bg-paper-entry rounded-lg border border-paper-border hover:-translate-y-0.5 hover:border-paper-accent/50 hover:shadow-sm transition-all duration-200",
            Link {
                class: "block group",
                to: Route::PostDetail { slug: post_slug },
                if has_cover {
                    div {
                        class: "mb-4 -mx-6 -mt-6 overflow-hidden rounded-t-lg",
                        ImageViewer {
                            src: post.cover_image.clone().unwrap_or_default(),
                            thumb_params: "?thumb=400x300",
                            alt: post.title.clone(),
                            lazy_load: true,
                        }
                    }
                }
                h2 {
                    class: "text-2xl font-bold leading-tight text-paper-primary group-hover:text-paper-accent transition-colors duration-200",
                    "{post.title}"
                }
                div {
                    class: "mt-2 text-sm text-paper-secondary leading-relaxed line-clamp-2",
                    "{post.summary.as_deref().unwrap_or_default()}"
                }
                div {
                    class: "mt-3 flex items-center gap-3 text-[13px] text-paper-secondary",
                    span { "{date_str}" }
                    if !post.tags.is_empty() {
                        span { "·" }
                        for tag in post.tags.clone().into_iter() {
                            span {
                                Link {
                                    class: "hover:text-paper-accent transition-colors duration-200",
                                    to: Route::TagDetail { tag: tag.clone() },
                                    onclick: move |evt: dioxus::events::MouseEvent| evt.stop_propagation(),
                                    "{tag}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

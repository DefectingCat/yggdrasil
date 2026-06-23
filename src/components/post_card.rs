//! 文章卡片组件
//!
//! 在首页、标签详情等列表中展示单篇文章的标题、摘要、封面、日期与标签。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::models::post::PostListItem;
use crate::router::Route;

/// 文章卡片组件。
///
/// Props：
/// - `post`：文章数据模型
///
/// 展示内容包括：
/// - 封面图（如有，使用 400x300 缩略图，不启用灯箱）
/// - 文章标题
/// - 摘要（最多两行）
/// - 发布日期与标签
///
/// 交互模型（采用覆盖层链接，避免 `<a>` 嵌套 `<a>` 的非法 HTML）：
/// - 整张卡片可点击跳转到文章详情：通过末尾一个绝对定位、铺满卡片的覆盖层 `Link` 实现。
/// - 标签是独立的 `Link`，通过 `relative z-10` 叠在覆盖层之上，并 `stop_propagation`，
///   点击标签进入标签详情页而不触发卡片跳转。
/// - 封面用裸 `.blur-img`（纯展示，无灯箱），点击走卡片跳转，避免交互歧义。
#[component]
pub fn PostCard(post: PostListItem) -> Element {
    let post_slug = post.slug.clone();
    let date_str = post.formatted_date();
    let cover_src = post.cover_image.clone().unwrap_or_default();
    let has_cover = post.cover_image.is_some();

    rsx! {
        article {
            class: "group relative mb-6 p-6 bg-paper-entry rounded-lg border border-paper-border hover:-translate-y-0.5 hover:border-paper-accent/50 hover:shadow-sm transition-all duration-200",
            if has_cover {
                div {
                    class: "mb-4 -mx-6 -mt-6 overflow-hidden rounded-t-lg",
                    div {
                        class: "blur-img post-card-cover-blur",
                        img {
                            class: "blur-img-placeholder",
                            src: "{cover_src}?w=20",
                            alt: "{post.title}",
                            loading: "lazy",
                        }
                        img {
                            class: "blur-img-full is-loaded",
                            src: "{cover_src}?thumb=400x300",
                            alt: "{post.title}",
                        }
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
                            // 标签叠在覆盖链接之上，点击进入标签详情页而非文章详情。
                            Link {
                                class: "relative z-10 hover:text-paper-accent transition-colors duration-200",
                                to: Route::TagDetail { tag: tag.clone() },
                                onclick: move |evt: dioxus::events::MouseEvent| evt.stop_propagation(),
                                "{tag}"
                            }
                        }
                    }
                }
            }
            // 覆盖层链接：铺满卡片，承担整卡跳转。z-0 位于标签 (z-10) 之下。
            Link {
                class: "absolute inset-0 z-0",
                aria_label: "post link to {post.title}",
                to: Route::PostDetail { slug: post_slug },
            }
        }
    }
}

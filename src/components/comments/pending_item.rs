//! 待审核评论项组件
//!
//! 展示用户刚提交、尚未通过审核的评论占位项，
//! 视觉上使用较低的透明度并标注"审核中"状态。

use dioxus::prelude::*;

use crate::hooks::comment_storage::{
    format_relative_time_iso, render_pending_content, PendingComment,
};

/// 待审核评论项组件。
///
/// Props：
/// - `comment`：待审核评论数据
/// - `post_id`：所属文章 ID（当前未使用，保留用于未来扩展）
///
/// 展示内容包括：作者头像/链接、基于创建时间动态计算的相对时间、审核中徽章、Markdown 渲染内容。
/// 深度最大展示 6 层缩进，孤儿评论深度会被修正为 0。
#[component]
#[allow(unused_variables)]
pub fn PendingCommentItem(comment: PendingComment, post_id: i32) -> Element {
    // 孤儿评论（parent_id 为 None 但 depth > 0）按顶层展示
    let depth = if comment.parent_id.is_none() && comment.depth > 0 {
        0
    } else {
        comment.depth
    };

    let indent = depth.min(6) * 24;
    let content_html = render_pending_content(&comment.content_md);
    // 基于创建时间实时计算相对时间，避免"刚刚"永久显示。
    let relative_time = format_relative_time_iso(&comment.created_at);

    // 作者名展示为链接或普通文本
    let author_element = match &comment.author_url {
        Some(url) if !url.is_empty() => rsx! {
            a {
                href: "{url}",
                rel: "nofollow noopener",
                target: "_blank",
                class: "font-medium text-paper-primary hover:text-paper-accent transition-colors",
                "{comment.author_name}"
            }
        },
        _ => rsx! {
            span { class: "font-medium text-paper-primary", "{comment.author_name}" }
        },
    };

    rsx! {
        div { class: "py-4 opacity-70", style: "margin-left: {indent}px",

            div { class: "flex gap-3",
                img {
                    src: "{comment.avatar_url}",
                    alt: "{comment.author_name} 的头像",
                    loading: "lazy",
                    decoding: "async",
                    class: "w-8 h-8 rounded-full shrink-0 mt-0.5 bg-gray-200 dark:bg-[#2a2a2a]",
                }

                div { class: "flex-1 min-w-0",
                    div { class: "flex items-center gap-1.5 text-sm mb-1.5 flex-wrap",
                        {author_element}
                        span { class: "text-paper-tertiary", "·" }
                        span {
                            class: "text-paper-tertiary",
                            title: "{comment.created_at}",
                            "{relative_time}"
                        }
                        span { class: "inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400",
                            "审核中"
                        }
                    }

                    div {
                        class: "prose prose-sm dark:prose-invert max-w-none text-paper-secondary",
                        dangerous_inner_html: "{content_html}",
                    }
                }
            }
        }
    }
}

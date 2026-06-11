use dioxus::prelude::*;

use crate::hooks::comment_storage::{PendingComment, render_pending_content};

#[component]
pub fn PendingCommentItem(comment: PendingComment, post_id: i32) -> Element {
    let _ = post_id;

    let depth = if comment.parent_id.is_none() && comment.depth > 0 {
        0
    } else {
        comment.depth
    };

    let indent = depth.min(6) * 24;
    let content_html = render_pending_content(&comment.content_md);

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
            span { class: "font-medium text-paper-primary",
                "{comment.author_name}"
            }
        },
    };

    rsx! {
        div {
            class: "py-4 opacity-70",
            style: "margin-left: {indent}px",

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
                            "刚刚"
                        }
                        span {
                            class: "inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400",
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

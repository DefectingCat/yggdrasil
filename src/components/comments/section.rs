use dioxus::prelude::*;

use crate::api::comments::{get_comments, CommentTreeResponse};
use crate::components::comments::form::CommentForm;
use crate::components::comments::list::CommentList;
use crate::components::skeletons::comment_skeleton::CommentListSkeleton;

#[derive(Clone, Copy)]
pub struct CommentContext {
    pub active_reply: Signal<Option<i64>>,
    pub refresh_trigger: Signal<bool>,
}

#[component]
pub fn CommentSection(post_id: i32) -> Element {
    let ctx = use_context_provider(|| CommentContext {
        active_reply: Signal::new(None),
        refresh_trigger: Signal::new(false),
    });

    let comments_resource = use_server_future(move || {
        let _ = ctx.refresh_trigger;
        get_comments(post_id)
    })?;

    let data = comments_resource.read();

    match data.as_ref().map(|r| r.as_ref()) {
        Some(Ok(CommentTreeResponse { comments, count })) => {
            let count = *count;
            rsx! {
                div { class: "space-y-8",
                    h2 { class: "text-xl font-bold text-paper-primary",
                        "评论区 ({count})"
                    }

                    CommentForm { post_id, parent_id: None }

                    if comments.is_empty() {
                        p { class: "text-paper-tertiary text-center py-8",
                            "暂无评论，成为第一个评论的人吧！"
                        }
                    } else {
                        CommentList { comments: comments.clone(), post_id }
                    }
                }
            }
        }
        Some(Err(_)) => {
            rsx! {
                div { class: "text-center text-red-500 dark:text-red-400 py-8",
                    "评论加载失败"
                }
            }
        }
        None => rsx! { CommentListSkeleton {} },
    }
}

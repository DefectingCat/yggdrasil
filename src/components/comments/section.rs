use dioxus::prelude::*;

use crate::api::comments::{check_pending_status, get_comments, CommentTreeResponse};
use crate::components::comments::form::CommentForm;
use crate::components::comments::list::CommentList;
use crate::components::skeletons::comment_skeleton::CommentListSkeleton;
use crate::hooks::comment_storage::{self, PendingComment};

#[derive(Clone, Copy)]
pub struct CommentContext {
    pub active_reply: Signal<Option<i64>>,
    pub refresh_trigger: Signal<bool>,
    pub pending_comments: Signal<Vec<PendingComment>>,
}

#[component]
pub fn CommentSection(post_id: i32) -> Element {
    let ctx = use_context_provider(|| {
        let pending: Vec<PendingComment> = comment_storage::load_pending_comments(post_id);
        comment_storage::prune_all_expired();

        CommentContext {
            active_reply: Signal::new(None),
            refresh_trigger: Signal::new(false),
            pending_comments: Signal::new(pending),
        }
    });

    use_future(move || {
        let pending = ctx.pending_comments;
        async move {
            let ids: Vec<i64> = pending().iter().map(|c| c.id).collect();
            if ids.is_empty() {
                return;
            }
            match check_pending_status(ids).await {
                Ok(statuses) => {
                    let to_remove: Vec<i64> = statuses
                        .into_iter()
                        .filter(|s| s.status != "pending")
                        .map(|s| s.id)
                        .collect();
                    if !to_remove.is_empty() {
                        comment_storage::remove_pending_ids(post_id, &to_remove);
                        pending.write().retain(|c| !to_remove.contains(&c.id));
                    }
                }
                Err(e) => {
                    tracing::warn!("check_pending_status failed: {}", e);
                }
            }
        }
    });

    let comments_resource = use_server_future(move || {
        let _ = ctx.refresh_trigger;
        get_comments(post_id)
    })?;

    let data = comments_resource.read();

    match data.as_ref().map(|r| r.as_ref()) {
        Some(Ok(CommentTreeResponse { comments, count })) => {
            let approved_count = *count;
            let pending_count = ctx.pending_comments.read().len() as i64;
            let total_count = approved_count + pending_count;
            let has_any = approved_count > 0 || pending_count > 0;
            rsx! {
                div { class: "space-y-8",
                    h2 { class: "text-xl font-bold text-paper-primary",
                        "评论区 ({total_count})"
                    }

                    CommentForm { post_id, parent_id: None }

                    if !has_any {
                        p { class: "text-paper-tertiary text-center py-8",
                            "暂无评论，成为第一个评论的人吧！"
                        }
                    } else {
                        CommentList {
                            comments: comments.clone(),
                            pending: ctx.pending_comments.read().clone(),
                            post_id,
                        }
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

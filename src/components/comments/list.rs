use dioxus::prelude::*;

use crate::models::comment::PublicComment;
use crate::hooks::comment_storage::PendingComment;
use crate::components::comments::item::CommentItem;
use crate::components::comments::pending_item::PendingCommentItem;

enum MergedComment {
    Approved(PublicComment),
    Pending(PendingComment),
}

fn merge_comments(
    approved: Vec<PublicComment>,
    pending: Vec<PendingComment>,
) -> Vec<MergedComment> {
    let mut merged: Vec<MergedComment> = approved
        .into_iter()
        .map(MergedComment::Approved)
        .chain(pending.into_iter().map(MergedComment::Pending))
        .collect();

    merged.sort_by(|a, b| {
        let time_a = match a {
            MergedComment::Approved(c) => c.created_at_iso.as_str(),
            MergedComment::Pending(c) => c.created_at.as_str(),
        };
        let time_b = match b {
            MergedComment::Approved(c) => c.created_at_iso.as_str(),
            MergedComment::Pending(c) => c.created_at.as_str(),
        };
        time_a.cmp(time_b)
    });

    merged
}

#[component]
pub fn CommentList(
    comments: Vec<PublicComment>,
    pending: Vec<PendingComment>,
    post_id: i32,
) -> Element {
    let merged = merge_comments(comments, pending);

    rsx! {
        div { class: "space-y-0 divide-y divide-gray-100 dark:divide-[#2a2a2a]",
            for item in merged {
                match item {
                    MergedComment::Approved(comment) => rsx! {
                        CommentItem { key: "{comment.id}", comment, post_id }
                    },
                    MergedComment::Pending(comment) => rsx! {
                        PendingCommentItem { key: "{comment.id}", comment, post_id }
                    },
                }
            }
        }
    }
}

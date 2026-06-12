use dioxus::prelude::*;

use crate::components::comments::item::CommentItem;
use crate::components::comments::pending_item::PendingCommentItem;
use crate::hooks::comment_storage::PendingComment;
use crate::models::comment::PublicComment;

#[derive(Clone)]
enum MergedComment {
    Approved(PublicComment),
    Pending(PendingComment),
}

fn merge_and_treeify(
    approved: Vec<PublicComment>,
    pending: Vec<PendingComment>,
) -> Vec<MergedComment> {
    use std::collections::{HashMap, HashSet};

    let all: Vec<MergedComment> = approved
        .into_iter()
        .map(MergedComment::Approved)
        .chain(pending.into_iter().map(MergedComment::Pending))
        .collect();

    let all_ids: HashSet<i64> = all
        .iter()
        .map(|c| match c {
            MergedComment::Approved(c) => c.id,
            MergedComment::Pending(c) => c.id,
        })
        .collect();

    let mut children_map: HashMap<Option<i64>, Vec<MergedComment>> = HashMap::new();
    for comment in all {
        let parent_id = match &comment {
            MergedComment::Approved(c) => c.parent_id,
            MergedComment::Pending(c) => c.parent_id,
        };
        let effective_parent = match parent_id {
            Some(pid) if !all_ids.contains(&pid) => None,
            _ => parent_id,
        };
        children_map
            .entry(effective_parent)
            .or_default()
            .push(comment);
    }

    for children in children_map.values_mut() {
        children.sort_by(|a, b| {
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
    }

    fn dfs(
        parent_id: Option<i64>,
        children_map: &HashMap<Option<i64>, Vec<MergedComment>>,
        result: &mut Vec<MergedComment>,
    ) {
        if let Some(children) = children_map.get(&parent_id) {
            for child in children {
                result.push(child.clone());
                let child_id = match child {
                    MergedComment::Approved(c) => Some(c.id),
                    MergedComment::Pending(c) => Some(c.id),
                };
                dfs(child_id, children_map, result);
            }
        }
    }

    let mut result = Vec::new();
    dfs(None, &children_map, &mut result);
    result
}

#[component]
pub fn CommentList(
    comments: Vec<PublicComment>,
    pending: Vec<PendingComment>,
    post_id: i32,
) -> Element {
    let merged = merge_and_treeify(comments, pending);

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

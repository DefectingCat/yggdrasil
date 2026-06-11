use dioxus::prelude::*;

use crate::models::comment::PublicComment;
use crate::components::comments::item::CommentItem;

#[component]
pub fn CommentList(comments: Vec<PublicComment>, post_id: i32) -> Element {
    rsx! {
        div { class: "space-y-0 divide-y divide-gray-100 dark:divide-[#2a2a2a]",
            for comment in comments {
                CommentItem { comment, post_id }
            }
        }
    }
}

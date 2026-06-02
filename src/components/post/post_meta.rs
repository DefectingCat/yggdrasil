use dioxus::prelude::*;

use crate::models::post::Post;

#[component]
pub fn PostMeta(post: Post) -> Element {
    rsx! {
        div { class: "post-meta",
            span { "{post.formatted_date()}" }
            span { "·" }
            span { "{post.reading_time} min read" }
            span { "·" }
            span { "{post.word_count} words" }
        }
    }
}

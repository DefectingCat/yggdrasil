use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::models::post::PostNav;
use crate::router::Route;

#[component]
pub fn PostNavLinks(prev: Option<PostNav>, next: Option<PostNav>) -> Element {
    rsx! {
        nav { class: "paginav",
            if let Some(prev_post) = prev {
                Link {
                    class: "prev",
                    to: Route::PostDetail { slug: prev_post.slug.clone() },
                    onclick: move |_evt: dioxus::events::MouseEvent| {},
                    span { class: "title", "« Prev" }
                    span { class: "post-title-nav", "{prev_post.title}" }
                }
            } else {
                span { class: "prev" }
            }

            if let Some(next_post) = next {
                Link {
                    class: "next",
                    to: Route::PostDetail { slug: next_post.slug.clone() },
                    onclick: move |_evt: dioxus::events::MouseEvent| {},
                    span { class: "title", "Next »" }
                    span { class: "post-title-nav", "{next_post.title}" }
                }
            } else {
                span { class: "next" }
            }
        }
    }
}

use dioxus::prelude::*;

use crate::models::post::PostNav;

#[component]
pub fn PostNavLinks(prev: Option<PostNav>, next: Option<PostNav>) -> Element {
    rsx! {
        nav { class: "paginav",
            if let Some(prev_post) = prev {
                a {
                    class: "prev",
                    href: "/post/{prev_post.slug}",
                    onclick: move |evt| {
                        evt.prevent_default();
                        dioxus::router::navigator().push(format!("/post/{}", prev_post.slug));
                    },
                    span { class: "title", "« Prev" }
                    span { class: "post-title-nav", "{prev_post.title}" }
                }
            } else {
                span { class: "prev" }
            }
            
            if let Some(next_post) = next {
                a {
                    class: "next",
                    href: "/post/{next_post.slug}",
                    onclick: move |evt| {
                        evt.prevent_default();
                        dioxus::router::navigator().push(format!("/post/{}", next_post.slug));
                    },
                    span { class: "title", "Next »" }
                    span { class: "post-title-nav", "{next_post.title}" }
                }
            } else {
                span { class: "next" }
            }
        }
    }
}

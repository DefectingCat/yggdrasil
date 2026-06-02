use dioxus::prelude::*;

use crate::components::post::post_nav_links::PostNavLinks;
use crate::models::post::Post;

#[component]
pub fn PostFooter(post: Post) -> Element {
    let tags = post.tags.clone();
    
    rsx! {
        footer { class: "post-footer",
            if !tags.is_empty() {
                ul { class: "post-tags",
                    for tag in tags.into_iter() {
                        li {
                            a {
                                href: "/tags/{tag}",
                                onclick: move |evt| {
                                    evt.prevent_default();
                                    dioxus::router::navigator().push(format!("/tags/{}", tag));
                                },
                                "{tag}"
                            }
                        }
                    }
                }
            }

            if post.prev_post.is_some() || post.next_post.is_some() {
                PostNavLinks { 
                    prev: post.prev_post,
                    next: post.next_post
                }
            }

            div { class: "back-to-home",
                button {
                    onclick: move |_| {
                        let _ = dioxus::router::navigator().push("/");
                    },
                    "← Back to Home"
                }
            }
        }
    }
}

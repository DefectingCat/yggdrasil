use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::components::post::post_nav_links::PostNavLinks;
use crate::models::post::Post;
use crate::router::Route;

#[component]
pub fn PostFooter(post: Post) -> Element {
    let tags = post.tags.clone();
    
    rsx! {
        footer { class: "post-footer",
            if !tags.is_empty() {
                ul { class: "post-tags",
                    for tag in tags.into_iter() {
                        li {
                            Link {
                                to: Route::TagDetail { tag: tag.clone() },
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
                Link {
                    to: Route::Home {},
                    "← Back to Home"
                }
            }
        }
    }
}

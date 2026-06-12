use dioxus::prelude::*;

use crate::components::post::breadcrumbs::Breadcrumbs;
use crate::components::post::post_meta::PostMeta;
use crate::models::post::{Post, PostStatus};

#[component]
pub fn PostHeader(post: Post) -> Element {
    rsx! {
        header { class: "post-header",
            Breadcrumbs { title: post.title.clone() }

            h1 { class: "post-title",
                "{post.title}"
                if post.status == PostStatus::Draft {
                    span {
                        class: "entry-hint",
                        title: "Draft",
                        svg {
                            xmlns: "http://www.w3.org/2000/svg",
                            height: "35",
                            view_box: "0 -960 960 960",
                            fill: "currentColor",
                            path {
                                d: "M160-410v-60h300v60H160Zm0-165v-60h470v60H160Zm0-165v-60h470v60H160Zm360 580v-123l221-220q9-9 20-13t22-4q12 0 23 4.5t20 13.5l37 37q9 9 13 20t4 22q0 11-4.5 22.5T862.09-380L643-160H520Zm300-263-37-37 37 37ZM580-220h38l121-122-18-19-19-18-122 121v38Zm141-141-19-18 37 37-18-19Z"
                            }
                        }
                    }
                }
            }

            if let Some(summary) = &post.summary {
                div { class: "post-description", "{summary}" }
            }

            PostMeta { post: post.clone() }
        }
    }
}

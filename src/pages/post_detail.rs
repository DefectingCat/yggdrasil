use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::posts::{get_post_by_slug, SinglePostResponse};
use crate::components::post::post_content::PostContent;
use crate::components::post::post_cover::PostCover;
use crate::components::post::post_footer::PostFooter;
use crate::components::post::post_header::PostHeader;
use crate::components::post::post_toc::PostToc;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::post_detail_skeleton::PostDetailSkeleton;
use crate::router::Route;

#[component]
pub fn PostDetail(slug: String) -> Element {
    let mut slug_signal = use_signal(|| slug.clone());
    if slug_signal() != slug {
        slug_signal.set(slug.clone());
    }

    let post = use_server_future(move || {
        let s = slug_signal();
        get_post_by_slug(s)
    })?;

    let post_data = post.read().as_ref().map(|r| match r {
        Ok(SinglePostResponse { post: Some(post) }) => Ok(post.clone()),
        Ok(SinglePostResponse { post: None }) => Err("not_found"),
        Err(_) => Err("error"),
    });

    match post_data {
        Some(Ok(post)) => {
            rsx! {
                article { class: "post-single",
                    PostHeader { post: post.clone() }

                    if let Some(cover) = &post.cover_image {
                        PostCover { src: cover.clone() }
                    }

                    if let Some(toc) = &post.toc_html {
                        PostToc { toc_html: toc.clone() }
                    }

                    PostContent {
                        content_html: post.content_html.clone().unwrap_or_default()
                    }

                    PostFooter { post: post.clone() }
                }
            }
        }
        Some(Err("not_found")) => {
            rsx! {
                div { class: "text-center py-20",
                    h2 { class: "text-2xl font-bold text-paper-primary mb-4",
                        "文章不存在"
                    }
                    p { class: "text-paper-secondary mb-6",
                        "这篇文章可能已被删除或移动。"
                    }
                    Link {
                        class: "px-6 py-2 bg-paper-primary text-paper-theme rounded-full font-medium hover:opacity-80 transition-opacity",
                        to: Route::Home {},
                        "返回首页"
                    }
                }
            }
        }
        Some(Err("error")) => {
            rsx! {
                div { class: "text-center text-red-500 dark:text-red-400 py-20",
                    "加载失败"
                }
            }
        }
        _ => {
            rsx! {
                DelayedSkeleton { PostDetailSkeleton {} }
            }
        }
    }
}

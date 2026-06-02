use dioxus::prelude::*;

use crate::api::posts::{get_post_by_slug, SinglePostResponse};
use crate::components::nav::use_nav_items;
use crate::components::page_layout::PageLayout;
use crate::components::post::post_content::PostContent;
use crate::components::post::post_cover::PostCover;
use crate::components::post::post_footer::PostFooter;
use crate::components::post::post_header::PostHeader;
use crate::components::post::post_toc::PostToc;
use crate::hooks::delayed_loading::use_delayed_loading;
use crate::router::Route;

#[component]
pub fn PostDetail(slug: String) -> Element {
    let route = use_route::<Route>();
    let slug_clone = slug.clone();
    let post_res = use_resource(move || get_post_by_slug(slug_clone.clone()));
    let nav_items = use_nav_items(route);
    let show_skeleton = use_delayed_loading(move || post_res.read().is_none());

    rsx! {
        PageLayout { nav_items,
            match &*post_res.read() {
                Some(Ok(SinglePostResponse { post: Some(post) })) => {
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
                Some(Ok(SinglePostResponse { post: None })) => {
                    rsx! {
                        div { class: "text-center py-20",
                            h2 { class: "text-2xl font-bold text-paper-primary mb-4",
                                "文章不存在"
                            }
                            p { class: "text-paper-secondary mb-6",
                                "这篇文章可能已被删除或移动。"
                            }
                            button {
                                class: "px-6 py-2 bg-paper-primary text-paper-theme rounded-full font-medium hover:opacity-80 transition-opacity",
                                onclick: move |_| {
                                    let _ = dioxus::router::navigator().push("/");
                                },
                                "返回首页"
                            }
                        }
                    }
                }
                Some(Err(e)) => {
                    rsx! {
                        div { class: "text-center text-red-500 dark:text-red-400 py-20",
                            "加载失败: {e}"
                        }
                    }
                }
                None => {
                    rsx! {
                        div { class: if show_skeleton() { "animate-pulse py-6 space-y-4" } else { "py-6 space-y-4 opacity-0" },
                            div { class: "h-10 w-3/4 bg-paper-tertiary rounded" }
                            div { class: "h-4 w-32 bg-paper-tertiary rounded" }
                            div { class: "h-4 w-full bg-paper-tertiary rounded mt-8" }
                            div { class: "h-4 w-full bg-paper-tertiary rounded" }
                            div { class: "h-4 w-2/3 bg-paper-tertiary rounded" }
                        }
                    }
                }
            }
        }
    }
}

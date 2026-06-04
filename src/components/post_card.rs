use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::models::post::Post;
use crate::router::Route;

#[component]
pub fn PostCard(post: Post) -> Element {
    let post_slug = post.slug.clone();
    let date_str = post.formatted_date();

    rsx! {
        article {
            class: "relative mb-6 p-6 bg-white dark:bg-[#2e2e33] rounded-lg border border-gray-200 dark:border-[#333] hover:-translate-y-0.5 hover:border-gray-300 dark:hover:border-gray-600 transition-all duration-250",
            Link {
                class: "block group",
                to: Route::PostDetail { slug: post_slug },
                h2 {
                    class: "text-2xl font-bold leading-tight text-gray-900 dark:text-[#dadadb] group-hover:opacity-80 transition-opacity",
                    "{post.title}"
                }
                div {
                    class: "mt-2 text-sm text-gray-500 dark:text-[#9b9c9d] leading-relaxed line-clamp-2",
                    "{post.summary.as_deref().unwrap_or_default()}"
                }
                div {
                    class: "mt-3 flex items-center gap-3 text-[13px] text-gray-400 dark:text-[#9b9c9d]",
                    span { "{date_str}" }
                    if !post.tags.is_empty() {
                        span { "·" }
                        for tag in post.tags.clone().into_iter() {
                            span {
                                Link {
                                    class: "hover:text-gray-600 dark:hover:text-[#dadadb] transition-colors",
                                    to: Route::TagDetail { tag: tag.clone() },
                                    onclick: move |evt: dioxus::events::MouseEvent| evt.stop_propagation(),
                                    "{tag}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

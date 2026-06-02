use dioxus::prelude::*;

use crate::models::post::Post;

#[component]
pub fn PostCard(post: Post) -> Element {
    let post_slug = post.slug.clone();
    let date_str = post.formatted_date();

    rsx! {
        article {
            class: "relative mb-6 p-6 bg-white dark:bg-[#2e2e33] rounded-lg border border-gray-200 dark:border-[#333] hover:-translate-y-0.5 hover:border-gray-300 dark:hover:border-gray-600 transition-all duration-250",
            a {
                class: "block group",
                href: "/post/{post_slug}",
                onclick: move |evt| {
                    evt.prevent_default();
                    dioxus::router::navigator().push(format!("/post/{}", post_slug).as_str());
                },
                h2 {
                    class: "text-2xl font-bold leading-tight text-gray-900 dark:text-[#dadadb] group-hover:opacity-80 transition-opacity",
                    "{post.title}"
                }
                div {
                    class: "mt-2 text-sm text-gray-500 dark:text-[#9b9c9d] leading-relaxed line-clamp-2",
                    "{post.summary.as_deref().unwrap_or(\"\")}"
                }
                div {
                    class: "mt-3 flex items-center gap-3 text-[13px] text-gray-400 dark:text-[#9b9c9d]",
                    span { "{date_str}" }
                    if !post.tags.is_empty() {
                        span { "·" }
                        for tag in post.tags.clone().into_iter() {
                            span {
                                a {
                                    class: "hover:text-gray-600 dark:hover:text-[#dadadb] transition-colors",
                                    href: "/tags/{tag}",
                                    onclick: move |evt| {
                                        evt.prevent_default();
                                        evt.stop_propagation();
                                        dioxus::router::navigator().push(format!("/tags/{}", tag).as_str());
                                    },
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

use dioxus::prelude::*;

use crate::pages::home::{Footer, Header, Post, POSTS};

#[derive(Clone, PartialEq)]
struct TagInfo {
    name: &'static str,
    count: usize,
}

fn collect_tags() -> Vec<TagInfo> {
    use std::collections::HashMap;

    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for post in POSTS.iter() {
        for tag in post.tags.iter() {
            *counts.entry(*tag).or_insert(0) += 1;
        }
    }

    let mut tags: Vec<TagInfo> = counts
        .into_iter()
        .map(|(name, count)| TagInfo { name, count })
        .collect();

    tags.sort_by(|a, b| a.name.cmp(b.name));
    tags
}

fn posts_for_tag(tag: &str) -> Vec<Post> {
    POSTS
        .iter()
        .filter(|p| p.tags.iter().any(|t| *t == tag))
        .cloned()
        .collect()
}

#[component]
pub fn TagsPage() -> Element {
    let tags = collect_tags();
    let total_posts = POSTS.len();

    rsx! {
        div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20] transition-colors duration-300",
            Header {}
            main { class: "flex-1 w-full max-w-3xl mx-auto px-6 py-6",
                header { class: "page-header mb-6",
                    h1 { class: "text-[34px] font-bold text-gray-900 dark:text-[#dadadb]",
                        "标签"
                    }
                    div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                        "共 "
                        span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{tags.len()}" }
                        " 个标签，"
                        span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{total_posts}" }
                        " 篇文章"
                    }
                }
                ul { class: "flex flex-wrap gap-4 mt-6",
                    for tag in tags.iter() {
                        li {
                            a {
                                class: "inline-flex items-center px-3 py-1.5 text-base font-medium bg-gray-100 dark:bg-[#2e2e33] text-gray-700 dark:text-[#9b9c9d] rounded-lg hover:bg-gray-200 dark:hover:bg-[#333] transition-colors",
                                href: "/tags/{tag.name}",
                                "{tag.name}"
                                sup { class: "ml-1 text-sm text-gray-500 dark:text-[#9b9c9d]", "{tag.count}" }
                            }
                        }
                    }
                }
            }
            Footer {}
        }
    }
}

#[component]
pub fn TagDetailPage(tag: String) -> Element {
    let posts = posts_for_tag(&tag);

    rsx! {
        div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20] transition-colors duration-300",
            Header {}
            main { class: "flex-1 w-full max-w-3xl mx-auto px-6 py-6",
                header { class: "page-header mb-6",
                    h1 { class: "text-[34px] font-bold text-gray-900 dark:text-[#dadadb]",
                        "{tag}"
                    }
                    div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                        "共 "
                        span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{posts.len()}" }
                        " 篇文章"
                    }
                }
                for post in posts.iter() {
                    TagPostEntry { post: post.clone() }
                }
            }
            Footer {}
        }
    }
}

#[component]
fn TagPostEntry(post: Post) -> Element {
    let tag_items = post.tags.iter().map(|t| *t).collect::<Vec<_>>();

    rsx! {
        article { class: "relative mb-6 p-6 bg-white dark:bg-[#2e2e33] rounded-lg border border-gray-200 dark:border-[#333] hover:-translate-y-0.5 hover:border-gray-300 dark:hover:border-gray-600 transition-all duration-250",
            a { class: "block group", href: "/post/{post.slug}",
                h2 { class: "text-2xl font-bold leading-tight text-gray-900 dark:text-[#dadadb] group-hover:opacity-80 transition-opacity",
                    "{post.title}"
                }
                div { class: "mt-2 text-sm text-gray-500 dark:text-[#9b9c9d] leading-relaxed line-clamp-2",
                    "{post.summary}"
                }
                div { class: "mt-3 flex items-center gap-3 text-[13px] text-gray-400 dark:text-[#9b9c9d]",
                    span { "{post.date}" }
                    span { "·" }
                    for (i, t) in tag_items.iter().enumerate() {
                        if i > 0 {
                            span { "," }
                        }
                        span {
                            a {
                                class: "hover:text-gray-600 dark:hover:text-[#dadadb] transition-colors",
                                href: "/tags/{t}",
                                "{t}"
                            }
                        }
                    }
                }
            }
        }
    }
}

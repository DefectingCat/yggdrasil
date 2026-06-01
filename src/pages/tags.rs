use dioxus::prelude::*;

use crate::api::posts::{get_posts_by_tag, list_tags, PostListResponse, TagListResponse};
use crate::components::header::{Header, NavItemConfig};
use crate::components::footer::Footer;
use crate::models::post::Post;
use crate::router::Route;
use crate::theme::ThemeToggle;

#[component]
pub fn Tags() -> Element {
    let route = use_route::<Route>();
    let tags_res = use_resource(list_tags);

    let nav_items = vec![
        NavItemConfig { href: "/", label: "首页", is_active: matches!(route, Route::Home {}) },
        NavItemConfig { href: "/archives", label: "归档", is_active: matches!(route, Route::Archives {}) },
        NavItemConfig { href: "/tags", label: "标签", is_active: matches!(route, Route::Tags {}) || matches!(route, Route::TagDetail { .. }) },
        NavItemConfig { href: "/search", label: "搜索", is_active: matches!(route, Route::Search {}) },
        NavItemConfig { href: "/about", label: "关于", is_active: matches!(route, Route::About {}) },
    ];

    let tags_data = use_memo(move || {
        match &*tags_res.read() {
            Some(Ok(TagListResponse { tags })) => Some(tags.clone()),
            _ => None,
        }
    });

    let total_posts = use_memo(move || {
        match &*tags_res.read() {
            Some(Ok(TagListResponse { tags })) => tags.iter().map(|t| t.post_count).sum::<i64>(),
            _ => 0,
        }
    });

    let has_error = use_memo(move || {
        matches!(&*tags_res.read(), Some(Err(_)))
    });

    rsx! {
        div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20] transition-colors duration-300",
            Header { nav_items, right_content: rsx! { ThemeToggle {} } }
            main { class: "flex-1 w-full max-w-3xl mx-auto px-6 py-6",
                header { class: "page-header mb-6",
                    h1 { class: "text-[34px] font-bold text-gray-900 dark:text-[#dadadb]",
                        "标签"
                    }
                    if tags_data().is_some() {
                        div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                            "共 "
                            span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{tags_data().unwrap().len()}" }
                            " 个标签，"
                            span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{total_posts()}" }
                            " 篇文章"
                        }
                    } else {
                        div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                            "加载中..."
                        }
                    }
                }
                if has_error() {
                    div { class: "text-center text-red-500 dark:text-red-400 py-20",
                        "加载失败"
                    }
                } else if tags_data().is_some() {
                    ul { class: "flex flex-wrap gap-4 mt-6",
                        for tag in tags_data().unwrap().into_iter() {
                            li {
                                a {
                                    class: "inline-flex items-center px-3 py-1.5 text-base font-medium bg-gray-100 dark:bg-[#2e2e33] text-gray-700 dark:text-[#9b9c9d] rounded-lg hover:bg-gray-200 dark:hover:bg-[#333] transition-colors",
                                    href: "/tags/{tag.name}",
                                    onclick: move |evt| {
                                        evt.prevent_default();
                                        dioxus::router::navigator().push(format!("/tags/{}", tag.name).as_str());
                                    },
                                    "{tag.name}"
                                    sup { class: "ml-1 text-sm text-gray-500 dark:text-[#9b9c9d]", "{tag.post_count}" }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "flex flex-wrap gap-4 mt-6 animate-pulse",
                        for _ in 0..8 {
                            div { class: "h-8 w-16 bg-gray-200 dark:bg-[#2a2a2a] rounded-lg" }
                        }
                    }
                }
            }
            Footer {}
        }
    }
}

#[component]
pub fn TagDetail(tag: String) -> Element {
    let route = use_route::<Route>();
    let tag_clone = tag.clone();
    let posts_res = use_resource(move || get_posts_by_tag(tag_clone.clone()));

    let nav_items = vec![
        NavItemConfig { href: "/", label: "首页", is_active: matches!(route, Route::Home {}) },
        NavItemConfig { href: "/archives", label: "归档", is_active: matches!(route, Route::Archives {}) },
        NavItemConfig { href: "/tags", label: "标签", is_active: matches!(route, Route::Tags {}) || matches!(route, Route::TagDetail { .. }) },
        NavItemConfig { href: "/search", label: "搜索", is_active: matches!(route, Route::Search {}) },
        NavItemConfig { href: "/about", label: "关于", is_active: matches!(route, Route::About {}) },
    ];

    let posts_data = use_memo(move || {
        match &*posts_res.read() {
            Some(Ok(PostListResponse { posts })) => Some(posts.clone()),
            _ => None,
        }
    });

    let post_count = use_memo(move || {
        match &*posts_res.read() {
            Some(Ok(PostListResponse { posts })) => posts.len(),
            _ => 0,
        }
    });

    let has_error = use_memo(move || {
        matches!(&*posts_res.read(), Some(Err(_)))
    });

    rsx! {
        div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20] transition-colors duration-300",
            Header { nav_items, right_content: rsx! { ThemeToggle {} } }
            main { class: "flex-1 w-full max-w-3xl mx-auto px-6 py-6",
                header { class: "page-header mb-6",
                    h1 { class: "text-[34px] font-bold text-gray-900 dark:text-[#dadadb]",
                        "{tag}"
                    }
                    if post_count() > 0 || has_error() {
                        div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                            "共 "
                            span { class: "font-medium text-gray-700 dark:text-[#dadadb]", "{post_count()}" }
                            " 篇文章"
                        }
                    } else {
                        div { class: "mt-2 text-base text-gray-500 dark:text-[#9b9c9d]",
                            "加载中..."
                        }
                    }
                }
                if has_error() {
                    div { class: "text-center text-red-500 dark:text-red-400 py-20",
                        "加载失败"
                    }
                } else if posts_data().is_some() {
                    for post in posts_data().unwrap().into_iter() {
                        TagPostEntry { post }
                    }
                } else {
                    div { class: "space-y-6 py-4 animate-pulse",
                        for _ in 0..3 {
                            div { class: "mb-6 p-6 bg-white dark:bg-[#2e2e33] rounded-lg border border-gray-200 dark:border-[#333]",
                                div { class: "h-7 w-3/4 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-3" }
                                div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded mb-2" }
                                div { class: "h-4 w-2/3 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
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
fn TagPostEntry(post: Post) -> Element {
    let post_slug = post.slug.clone();
    let date_str = post
        .published_at
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| post.created_at.format("%Y-%m-%d").to_string());

    rsx! {
        article { class: "relative mb-6 p-6 bg-white dark:bg-[#2e2e33] rounded-lg border border-gray-200 dark:border-[#333] hover:-translate-y-0.5 hover:border-gray-300 dark:hover:border-gray-600 transition-all duration-250",
            a {
                class: "block group",
                href: "/post/{post_slug}",
                onclick: move |evt| {
                    evt.prevent_default();
                    dioxus::router::navigator().push(format!("/post/{}", post_slug).as_str());
                },
                h2 { class: "text-2xl font-bold leading-tight text-gray-900 dark:text-[#dadadb] group-hover:opacity-80 transition-opacity",
                    "{post.title}"
                }
                div { class: "mt-2 text-sm text-gray-500 dark:text-[#9b9c9d] leading-relaxed line-clamp-2",
                    "{post.summary.as_deref().unwrap_or(\"\")}"
                }
                div { class: "mt-3 flex items-center gap-3 text-[13px] text-gray-400 dark:text-[#9b9c9d]",
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

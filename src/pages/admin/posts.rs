use dioxus::prelude::*;
use dioxus::router::components::Link;

#[cfg(target_arch = "wasm32")]
use crate::api::posts::list_posts;
#[cfg(target_arch = "wasm32")]
use crate::api::posts::PostListResponse;
use crate::api::posts::{delete_post, rebuild_content_html, CreatePostResponse};
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::posts_skeleton::PostsSkeleton;
use crate::models::post::Post;
use crate::router::Route;

const POSTS_PER_PAGE: i32 = 20;

#[component]
pub fn Posts() -> Element {
    rsx! { PostsPage { page: 1 } }
}

#[component]
pub fn PostsPage(page: i32) -> Element {
    let current_page = page.max(1);
    let mut posts = use_signal(Vec::new);
    let mut total = use_signal(|| 0_i64);
    let mut loading = use_signal(|| true);
    let mut deleting = use_signal(|| None::<i32>);
    let mut rebuilding = use_signal(|| false);
    let mut rebuild_result = use_signal(|| Option::<String>::None);

    use_effect(move || {
        let _ = current_page;

        loading.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            let p = current_page;
            spawn(async move {
                match list_posts(p, POSTS_PER_PAGE).await {
                    Ok(PostListResponse { posts: list, total: t }) => {
                        posts.set(list);
                        total.set(t);
                    }
                    Err(_) => {}
                }
                loading.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            loading.set(false);
        }
    });

    let get_posts = move || -> Vec<Post> { posts() };

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold text-gray-900 dark:text-[#dadadb]",
                    "文章管理"
                }
                div { class: "flex items-center gap-3",
                    div { class: "group relative",
                        button {
                            class: if rebuilding() {
                                "px-4 py-2 rounded-full text-sm font-medium cursor-not-allowed text-gray-400 dark:text-[#666] border border-gray-300 dark:border-[#444]"
                            } else {
                                "px-4 py-2 rounded-full text-sm font-medium cursor-pointer text-gray-700 dark:text-[#b0b0b1] border border-gray-300 dark:border-[#444] hover:border-gray-900 dark:hover:border-[#dadadb] hover:text-gray-900 dark:hover:text-[#dadadb] transition-all"
                            },
                            disabled: rebuilding(),
                            onclick: move |_| {
                                rebuilding.set(true);
                                rebuild_result.set(None);
                                spawn(async move {
                                    match rebuild_content_html(false).await {
                                        Ok(count) => {
                                            rebuild_result.set(Some(format!("已重建 {count} 篇文章")));
                                        }
                                        Err(e) => {
                                            rebuild_result.set(Some(format!("失败: {e}")));
                                        }
                                    }
                                    rebuilding.set(false);
                                });
                            },
                            if rebuilding() { "重建中..." } else { "重建内容" }
                        }
                        div { class: "pointer-events-none absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-1.5 text-xs font-medium whitespace-nowrap rounded-lg opacity-0 group-hover:opacity-100 transition-opacity duration-200 bg-gray-900 dark:bg-white text-white dark:text-gray-900 shadow-lg",
                            "重建 content_html 为空的文章渲染缓存"
                        }
                    }
                    Link {
                        class: "px-4 py-2 bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full text-sm font-medium hover:opacity-80 transition-opacity cursor-pointer",
                        to: Route::Write {},
                        "+ 写文章"
                    }
                }
            }

            if let Some(msg) = rebuild_result() {
                div { class: "text-sm text-gray-600 dark:text-[#9b9c9d] px-1",
                    "{msg}"
                }
            }

            if loading() && posts().is_empty() {
                DelayedSkeleton { PostsSkeleton {} }
            } else if posts().is_empty() {
                div { class: "text-center py-20 text-gray-500 dark:text-[#9b9c9d]",
                    "暂无文章"
                }
            } else {
                div { class: "bg-white dark:bg-[#2e2e33] rounded-xl border border-gray-200 dark:border-[#333] overflow-hidden",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "border-b border-gray-200 dark:border-[#333] text-left text-gray-500 dark:text-[#9b9c9d]",
                                    th { class: "px-4 py-3 font-medium", "标题" }
                                    th { class: "px-4 py-3 font-medium w-24 text-center", "状态" }
                                    th { class: "px-4 py-3 font-medium w-32", "日期" }
                                    th { class: "px-4 py-3 font-medium w-24 text-right", "操作" }
                                }
                            }
                            tbody {
                                for post in get_posts().iter() {
                                    PostRow {
                                        post: post.clone(),
                                        deleting: deleting() == Some(post.id),
                                        on_delete: move |id| {
                                            deleting.set(Some(id));
                                            let id_for_api = id;
                                            posts.with_mut(|list| list.retain(|p| p.id != id));
                                            total.with_mut(|t| *t = t.saturating_sub(1));
                                            spawn(async move {
                                                match delete_post(id_for_api).await {
                                                    Ok(CreatePostResponse { success: false, message: _message, .. }) => {
                                                        #[cfg(target_arch = "wasm32")]
                                                        web_sys::window().map(|w| w.alert_with_message(&_message).ok());
                                                    }
                                                    Err(_e) => {
                                                        #[cfg(target_arch = "wasm32")]
                                                        web_sys::window().map(|w| w.alert_with_message("删除失败").ok());
                                                    }
                                                    _ => {}
                                                }
                                                deleting.set(None);
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Pagination { current_page, total: total() }
            }
        }
    }
}

#[component]
fn Pagination(current_page: i32, total: i64) -> Element {
    let has_prev = current_page > 1;
    let total_pages = ((total + POSTS_PER_PAGE as i64 - 1) / POSTS_PER_PAGE as i64).max(1) as i32;
    let has_next = current_page < total_pages;

    rsx! {
        nav { class: "flex mt-6 justify-between",
            if has_prev {
                Link {
                    class: "inline-flex items-center px-4 py-2 text-sm text-white bg-gray-900 dark:bg-[#dadadb] dark:text-gray-900 rounded-full hover:opacity-80 transition-opacity cursor-pointer",
                    to: if current_page - 1 <= 1 {
                        Route::Posts {}
                    } else {
                        Route::PostsPage { page: current_page - 1 }
                    },
                    span { class: "mr-1", "«" }
                    "上一页"
                }
            } else {
                span { class: "inline-flex items-center px-4 py-2 text-sm text-gray-400 bg-gray-100 dark:bg-[#2a2a2a] rounded-full cursor-not-allowed",
                    span { class: "mr-1", "«" }
                    "上一页"
                }
            }
            span { class: "text-sm text-gray-500 dark:text-[#9b9c9d] self-center",
                "{current_page} / {total_pages} 页 (共 {total} 篇)"
            }
            if has_next {
                Link {
                    class: "inline-flex items-center px-4 py-2 text-sm text-white bg-gray-900 dark:bg-[#dadadb] dark:text-gray-900 rounded-full hover:opacity-80 transition-opacity cursor-pointer",
                    to: Route::PostsPage { page: current_page + 1 },
                    "下一页"
                    span { class: "ml-1", "»" }
                }
            } else {
                span { class: "inline-flex items-center px-4 py-2 text-sm text-gray-400 bg-gray-100 dark:bg-[#2a2a2a] rounded-full cursor-not-allowed",
                    "下一页"
                    span { class: "ml-1", "»" }
                }
            }
        }
    }
}

#[component]
fn PostRow(post: Post, deleting: bool, on_delete: EventHandler<i32>) -> Element {
    let date_str = post.formatted_date();
    let status_label = post.status_label();
    let status_badge_class = post.status_badge_class();

    rsx! {
        tr { class: "border-b border-gray-100 dark:border-[#333] last:border-0 hover:bg-gray-50 dark:hover:bg-[#2a2a2a] transition-colors",
            td { class: "px-4 py-3",
                Link {
                    class: "text-gray-900 dark:text-[#dadadb] hover:opacity-80 transition-opacity",
                    to: Route::PostDetail { slug: post.slug.clone() },
                    "{post.title}"
                }
            }
            td { class: "px-4 py-3 text-center",
                span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {status_badge_class}",
                    "{status_label}"
                }
            }
            td { class: "px-4 py-3 text-gray-500 dark:text-[#9b9c9d]",
                "{date_str}"
            }
            td { class: "px-4 py-3 text-right",
                div { class: "flex justify-end gap-3",
                    Link {
                        class: "text-xs text-gray-600 dark:text-[#9b9c9d] hover:text-gray-900 dark:hover:text-[#dadadb] transition-colors cursor-pointer",
                        to: Route::WriteEdit { id: post.id },
                        "编辑"
                    }
                    button {
                        class: if deleting {
                            "text-xs text-gray-400 cursor-not-allowed"
                        } else {
                            "text-xs text-red-500 hover:text-red-700 dark:hover:text-red-300 transition-colors cursor-pointer"
                        },
                        disabled: deleting,
                        onclick: move |_| on_delete.call(post.id),
                        if deleting { "删除中..." } else { "删除" }
                    }
                }
            }
        }
    }
}

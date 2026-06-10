use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::posts::{delete_post, list_posts, CreatePostResponse, PostListResponse};
use crate::hooks::delayed_loading::use_delayed_loading;
use crate::models::post::Post;
use crate::router::Route;

#[component]
#[allow(unused_variables)]
pub fn Posts() -> Element {
    let mut posts_res = use_resource(list_posts);
    let mut deleting = use_signal(|| None::<i32>);
    let show_skeleton = use_delayed_loading(move || posts_res.read().is_none());

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold text-gray-900 dark:text-[#dadadb]",
                    "文章管理"
                }
                Link {
                    class: "px-4 py-2 bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full text-sm font-medium hover:opacity-80 transition-opacity cursor-pointer",
                    to: Route::Write {},
                    "+ 写文章"
                }
            }

            match &*posts_res.read() {
                Some(Ok(PostListResponse { posts, .. })) => {
                    if posts.is_empty() {
                        rsx! {
                            div { class: "text-center py-20 text-gray-500 dark:text-[#9b9c9d]",
                                "暂无文章"
                            }
                        }
                    } else {
                        rsx! {
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
                                        for post in posts.iter() {
                                            PostRow {
                                                post: post.clone(),
                                                deleting: deleting() == Some(post.id),
                                                on_delete: move |id| {
                                                    deleting.set(Some(id));
                                                    spawn(async move {
                                                        match delete_post(id).await {
                                                            Ok(CreatePostResponse { success: true, .. }) => {
                                                                posts_res.restart();
                                                            }
                                                            Ok(CreatePostResponse { success: false, message, .. }) => {
                                                                #[cfg(target_arch = "wasm32")]
                                                                web_sys::window().map(|w| w.alert_with_message(&message).ok());
                                                            }
                                                            Err(_e) => {
                                                                #[cfg(target_arch = "wasm32")]
                                                                web_sys::window().map(|w| w.alert_with_message("删除失败").ok());
                                                            }
                                                        }
                                                        deleting.set(None);
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(_e)) => {
                    rsx! {
                        div { class: "text-center text-red-500 dark:text-red-400 py-20",
                            "加载失败"
                        }
                    }
                }
                None => {
                    rsx! {
                        div { class: if show_skeleton() { "bg-white dark:bg-[#2e2e33] rounded-xl border border-gray-200 dark:border-[#333] animate-pulse" } else { "bg-white dark:bg-[#2e2e33] rounded-xl border border-gray-200 dark:border-[#333] opacity-0" },
                            for _ in 0..5 {
                                div { class: "flex items-center px-4 py-3 border-b border-gray-100 dark:border-[#333] last:border-0",
                                    div { class: "h-4 w-1/3 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                    div { class: "ml-auto h-4 w-16 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                }
                            }
                        }
                    }
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
            td { class: "px-4 py-3 text-right flex justify-end gap-3",
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

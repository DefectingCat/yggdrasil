use dioxus::prelude::*;

use crate::api::posts::{delete_post, list_posts, CreatePostResponse, PostListResponse};
use crate::components::suspense_wrapper::SuspenseWrapper;
use crate::models::post::{Post, PostStatus};

#[component]
pub fn Posts() -> Element {
    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold text-gray-900 dark:text-[#dadadb]",
                    "文章管理"
                }
                button {
                    class: "px-4 py-2 bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full text-sm font-medium hover:opacity-80 transition-opacity cursor-pointer",
                    onclick: move |_| {
                        dioxus::router::navigator().push("/admin/write");
                    },
                    "+ 写文章"
                }
            }
            SuspenseWrapper {
                PostsTable {}
            }
        }
    }
}

#[component]
fn PostsTable() -> Element {
    let mut refresh = use_signal(|| 0);
    let mut deleting = use_signal(|| None::<i32>);
    let posts_res = use_server_future(move || {
        let _ = refresh();
        list_posts()
    })?;

    let posts_data = posts_res.read().as_ref().map(|r| match r {
        Ok(PostListResponse { posts }) => Ok(posts.clone()),
        Err(e) => Err(e.to_string()),
    });

    match posts_data {
        Some(Ok(posts)) => {
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
                                                        refresh.set(refresh() + 1);
                                                    }
                                                    Ok(CreatePostResponse { success: false, message, .. }) => {
                                                        #[cfg(target_arch = "wasm32")]
                                                        web_sys::window().map(|w| w.alert_with_message(&message).ok());
                                                    }
                                                    Err(e) => {
                                                        #[cfg(target_arch = "wasm32")]
                                                        web_sys::window().map(|w| w.alert_with_message(&format!("删除失败: {}", e)).ok());
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
        Some(Err(e)) => {
            rsx! {
                div { class: "text-center text-red-500 dark:text-red-400 py-20",
                    "加载失败: {e}"
                }
            }
        }
        _ => {
            rsx! {
                div { class: "text-center text-gray-500 dark:text-[#9b9c9d] py-20",
                    "加载中..."
                }
            }
        }
    }
}

#[component]
fn PostRow(post: Post, deleting: bool, on_delete: EventHandler<i32>) -> Element {
    let date_str = post
        .published_at
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| post.created_at.format("%Y-%m-%d").to_string());

    let (status_label, status_class) = if post.status == PostStatus::Published {
        (
            "已发布",
            "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300",
        )
    } else {
        (
            "草稿",
            "bg-gray-100 dark:bg-[#333] text-gray-600 dark:text-[#9b9c9d]",
        )
    };

    rsx! {
        tr { class: "border-b border-gray-100 dark:border-[#333] last:border-0 hover:bg-gray-50 dark:hover:bg-[#2a2a2a] transition-colors",
            td { class: "px-4 py-3",
                a {
                    class: "text-gray-900 dark:text-[#dadadb] hover:opacity-80 transition-opacity",
                    href: "/post/{post.slug}",
                    onclick: move |evt| {
                        evt.prevent_default();
                        dioxus::router::navigator().push(format!("/post/{}", post.slug).as_str());
                    },
                    "{post.title}"
                }
            }
            td { class: "px-4 py-3 text-center",
                span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {status_class}",
                    "{status_label}"
                }
            }
            td { class: "px-4 py-3 text-gray-500 dark:text-[#9b9c9d]",
                "{date_str}"
            }
            td { class: "px-4 py-3 text-right",
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

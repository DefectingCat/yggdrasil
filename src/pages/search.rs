use dioxus::prelude::*;

use crate::api::posts::{search_posts, PostListResponse};
use crate::components::nav::use_nav_items;
use crate::components::page_layout::PageLayout;
use crate::components::post_card::PostCard;
use crate::hooks::delayed_loading::use_delayed_loading;
use crate::router::Route;

#[component]
pub fn Search() -> Element {
    let route = use_route::<Route>();
    let mut query = use_signal(|| "".to_string());
    let mut search_res = use_signal(|| None::<Result<PostListResponse, ServerFnError>>);
    let mut is_searching = use_signal(|| false);
    let nav_items = use_nav_items(route);
    let show_skeleton = use_delayed_loading(move || is_searching());

    let mut on_search = move || {
        let q = query().trim().to_string();
        if q.is_empty() {
            return;
        }
        is_searching.set(true);
        search_res.set(None);
        spawn(async move {
            let res = search_posts(q).await;
            search_res.set(Some(res));
            is_searching.set(false);
        });
    };

    rsx! {
        PageLayout { nav_items,
            header { class: "page-header mb-6",
                h1 { class: "text-[34px] font-bold text-gray-900 dark:text-[#dadadb]",
                    "搜索"
                }
            }
            div { class: "mb-8",
                div { class: "flex gap-2",
                    input {
                        class: "flex-1 px-4 py-2 border border-gray-200 dark:border-[#333] rounded-lg bg-white dark:bg-[#2e2e33] text-gray-900 dark:text-[#dadadb] focus:outline-none focus:border-gray-400 dark:focus:border-gray-600",
                        r#type: "text",
                        placeholder: "输入关键词搜索文章...",
                        value: query(),
                        oninput: move |e| query.set(e.value()),
                        onkeydown: move |e| if e.key() == Key::Enter { on_search() },
                    }
                    button {
                        class: "px-6 py-2 bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full font-medium hover:opacity-80 transition-opacity",
                        onclick: move |_| on_search(),
                        "搜索"
                    }
                }
            }
            if is_searching() {
                div { class: if show_skeleton() { "space-y-6 py-4 animate-pulse" } else { "space-y-6 py-4 opacity-0" },
                    for _ in 0..3 {
                        div { class: "mb-6 p-6 bg-white dark:bg-[#2e2e33] rounded-lg border border-gray-200 dark:border-[#333]",
                            div { class: "h-7 w-3/4 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-3" }
                            div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded mb-2" }
                            div { class: "h-4 w-2/3 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                        }
                    }
                }
            } else if let Some(Ok(PostListResponse { posts })) = search_res() {
                if posts.is_empty() {
                    div { class: "text-center text-gray-500 dark:text-[#9b9c9d] py-20",
                        "未找到相关文章"
                    }
                } else {
                    for post in posts.iter() {
                        PostCard { post: post.clone() }
                    }
                }
            } else if let Some(Err(e)) = search_res() {
                div { class: "text-center text-red-500 dark:text-red-400 py-20",
                    "搜索失败: {e}"
                }
            }
        }
    }
}

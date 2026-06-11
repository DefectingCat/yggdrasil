use dioxus::prelude::*;

use crate::api::posts::{search_posts, PostListResponse};
use crate::components::post_card::PostCard;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::search_skeleton::SearchSkeleton;

#[component]
pub fn Search() -> Element {
    let mut query = use_signal(|| "".to_string());
    let mut search_res = use_signal(|| None::<Result<PostListResponse, ServerFnError>>);
    let mut is_searching = use_signal(|| false);
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
        header { class: "page-header mb-6",
            h1 { class: "text-4xl font-bold text-paper-primary tracking-tight",
                "搜索"
            }
        }
        div { class: "mb-8",
            div { class: "flex gap-2",
                input {
                    class: "flex-1 px-4 py-2 border border-paper-border rounded-lg bg-paper-entry text-paper-primary placeholder:text-paper-tertiary focus:outline-none focus:border-paper-accent focus:ring-1 focus:ring-paper-accent/30",
                    r#type: "text",
                    placeholder: "输入关键词搜索文章...",
                    value: query(),
                    oninput: move |e| query.set(e.value()),
                    onkeydown: move |e| if e.key() == Key::Enter { on_search() },
                }
                button {
                    class: "px-6 py-2 bg-paper-accent text-white rounded-full font-medium hover:brightness-110 active:scale-[0.98] transition-all duration-200",
                    onclick: move |_| on_search(),
                    "搜索"
                }
            }
        }
        if is_searching() {
            DelayedSkeleton { SearchSkeleton {} }
        } else if let Some(Ok(PostListResponse { posts, total: _ })) = search_res() {
            if posts.is_empty() {
                div { class: "text-center text-paper-secondary py-20",
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

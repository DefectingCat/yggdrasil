//! 搜索页面模块。
//!
//! 对应路由 `/search`。
//!
//! 数据获取：用户在输入框中键入关键词并触发搜索后，
//! 通过 Dioxus 的 `spawn` 在本地启动异步任务，调用 `search_posts` server function。
//! 与首页/归档不同，搜索是交互式客户端行为，不在服务端渲染阶段预取数据。
//! 在 `wasm32` 目标下，该 server function 的函数体被替换为向服务端端点发起 HTTP POST 请求的客户端存根；
//! 实际的数据库访问逻辑仅在 `feature = "server"` 启用时运行。

use dioxus::prelude::*;

use crate::api::posts::{search_posts, PostListResponse};
use crate::components::post_card::PostCard;
use crate::components::empty_state::EmptyState;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::search_skeleton::SearchSkeleton;

/// 搜索页面组件，对应路由 `/search`。
///
/// 维护搜索关键词、搜索结果与加载状态，渲染搜索框与结果列表。
/// 结果通过客户端异步请求获取，而非 `use_server_future` 预取。
#[component]
pub fn Search() -> Element {
    // 当前输入框中的搜索关键词。
    let mut query = use_signal(|| "".to_string());
    // 搜索结果：None 表示尚未执行搜索或已清空。
    let mut search_res = use_signal(|| None::<Result<PostListResponse, ServerFnError>>);
    // 是否正在发起搜索请求。
    let mut is_searching = use_signal(|| false);
    // 触发搜索的回调：校验空查询后启动异步请求。
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
            h1 { class: "text-4xl font-bold text-paper-primary tracking-tight", "搜索" }
        }
        div { class: "mb-8",
            div { class: "flex gap-2",
                input {
                    class: "flex-1 px-4 py-2 border border-paper-border rounded-lg bg-paper-entry text-paper-primary placeholder:text-paper-tertiary focus:outline-none focus:border-paper-accent focus:ring-1 focus:ring-paper-accent/30",
                    r#type: "text",
                    placeholder: "输入关键词搜索文章...",
                    value: query(),
                    oninput: move |e| query.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            on_search()
                        }
                    },
                }
                button {
                    class: "px-6 py-2 bg-paper-accent text-white rounded-full font-medium hover:brightness-110 active:scale-[0.98] transition-all duration-200",
                    onclick: move |_| on_search(),
                    "搜索"
                }
            }
        }
        // 根据搜索状态展示骨架屏、结果列表、空状态或错误提示。
        if is_searching() {
            DelayedSkeleton { SearchSkeleton {} }
        } else if let Some(Ok(PostListResponse { posts, total: _ })) = search_res() {
            if posts.is_empty() {
                EmptyState {
                    title: "未找到相关文章",
                    description: "换个关键词再试一次吧",
                }
            } else {
                for post in posts.iter() {
                    PostCard { key: "{post.id}", post: post.clone() }
                }
            }
        } else if search_res().as_ref().map(|r| r.is_err()).unwrap_or(false) {
            div { class: "text-center text-red-500 dark:text-red-400 py-20", "搜索失败" }
        } else {
            div { class: "flex flex-col items-center justify-center mt-24 mb-12 page-enter",
                img {
                    class: "w-56 h-auto rounded-lg select-none dark:brightness-90",
                    src: "/images/xiantiaoxiaogou_02.webp",
                    alt: "空状态提示",
                    draggable: "false",
                }
            }
        }
    }
}

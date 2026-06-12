//! 标签页面模块。
//!
//! 对应路由：
//! - `/tags`：标签云，展示所有标签及关联文章数量。
//! - `/tags/:tag`：标签详情页，展示指定标签下的已发布文章列表。
//!
//! 数据获取：
//! - 标签云通过 `use_server_future(list_tags)` 获取全部标签信息。
//! - 标签详情通过 `use_server_future` 调用 `get_posts_by_tag(tag)` 获取该标签下的文章列表。
//! 在 `wasm32` 目标下，这些 server function 的函数体被替换为向服务端端点发起 HTTP POST 请求的客户端存根；
//! 实际的数据库访问逻辑仅在 `feature = "server"` 启用时运行。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::posts::{get_posts_by_tag, list_tags, PostListResponse, TagListResponse};
use crate::components::post_card::PostCard;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::tags_skeleton::{TagDetailSkeleton, TagsSkeleton};
use crate::router::Route;

/// 标签云页面组件，对应路由 `/tags`。
///
/// 渲染页面标题，并委托给 `TagsContent` 展示所有标签。
#[component]
pub fn Tags() -> Element {
    rsx! {
        header { class: "page-header mb-6",
            h1 { class: "text-4xl font-bold text-paper-primary tracking-tight",
                "标签"
            }
        }
        TagsContent {}
    }
}

/// 标签云内容组件。
///
/// 通过 `use_server_future(list_tags)` 异步获取标签列表；
/// 成功时渲染标签总数、文章总数以及每个标签的链接。
#[component]
fn TagsContent() -> Element {
    let tags_res = use_server_future(list_tags)?;

    // 将结果映射为仅包含标签列表的形式。
    let tags_data = tags_res.read().as_ref().map(|r| match r {
        Ok(TagListResponse { tags }) => Ok(tags.clone()),
        Err(e) => Err(e.to_string()),
    });

    match tags_data {
        Some(Ok(tags)) => {
            let total = tags.iter().map(|t| t.post_count).sum::<i64>();
            rsx! {
                div { class: "mt-2 text-base text-paper-secondary",
                    "共 "
                    span { class: "font-medium text-paper-primary", "{tags.len()}" }
                    " 个标签，"
                    span { class: "font-medium text-paper-primary", "{total}" }
                    " 篇文章"
                }
                ul { class: "flex flex-wrap gap-4 mt-6",
                    for tag in tags {
                        li {
                            Link {
                                class: "inline-flex items-center px-3 py-1.5 text-base font-medium bg-paper-accent-soft text-paper-accent rounded-lg hover:bg-paper-accent hover:text-white transition-all duration-200",
                                to: Route::TagDetail { tag: tag.name.clone() },
                                "{tag.name}"
                                sup { class: "ml-1 text-sm text-paper-secondary", "{tag.post_count}" }
                            }
                        }
                    }
                }
            }
        }
        Some(Err(_)) => {
            rsx! {
                div { class: "text-center text-red-500 dark:text-red-400 py-20",
                    "加载失败"
                }
            }
        }
        _ => {
            rsx! {
                DelayedSkeleton { TagsSkeleton {} }
            }
        }
    }
}

/// 标签详情页面组件，对应路由 `/tags/:tag`。
///
/// 渲染当前标签名称，并委托给 `TagDetailContent` 展示该标签下的文章列表。
#[component]
pub fn TagDetail(tag: String) -> Element {
    rsx! {
        header { class: "page-header mb-6",
            h1 { class: "text-4xl font-bold text-paper-primary tracking-tight",
                "{tag}"
            }
        }
        TagDetailContent { tag: tag.clone() }
    }
}

/// 标签详情内容组件。
///
/// 通过 `use_server_future` 调用 `get_posts_by_tag` 获取指定标签下的文章；
/// 成功时渲染文章总数与文章卡片。
#[component]
fn TagDetailContent(tag: String) -> Element {
    let posts_res = use_server_future(move || get_posts_by_tag(tag.clone()))?;

    // 将结果映射为 (posts, total) 形式以便渲染。
    let posts_data = posts_res.read().as_ref().map(|r| match r {
        Ok(PostListResponse { posts, total }) => Ok((posts.clone(), *total)),
        Err(e) => Err(e.to_string()),
    });

    match posts_data {
        Some(Ok((posts, total))) => {
            rsx! {
                div { class: "mt-2 text-base text-paper-secondary",
                    "共 "
                    span { class: "font-medium text-paper-primary", "{total}" }
                    " 篇文章"
                }
                for post in posts.iter() {
                    PostCard { post: post.clone() }
                }
            }
        }
        Some(Err(_)) => {
            rsx! {
                div { class: "text-center text-red-500 dark:text-red-400 py-20",
                    "加载失败"
                }
            }
        }
        _ => {
            rsx! {
                DelayedSkeleton { TagDetailSkeleton {} }
            }
        }
    }
}

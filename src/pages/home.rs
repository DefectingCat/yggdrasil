//! 首页模块。
//!
//! 对应路由：
//! - `/`：首页，默认展示第 1 页文章。
//! - `/page/:page`：分页首页，展示指定页码的已发布文章列表。
//!
//! 数据获取：通过 `use_server_future` 调用 `list_published_posts` server function，
//! 从服务端获取已发布文章的分页列表与总数，并渲染文章卡片与分页导航。
//! 在 `wasm32` 目标下，server function 的函数体被替换为向服务端端点发起 HTTP POST 请求的客户端存根；
//! 实际的数据库访问逻辑仅在 `feature = "server"` 启用时运行。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::posts::{list_published_posts, PostListResponse};
use crate::components::post_card::PostCard;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::home_skeleton::HomeSkeleton;
use crate::router::Route;

// 每页展示的已发布文章数量，用于分页计算。
const POSTS_PER_PAGE: i32 = 10;

/// 首页组件，对应路由 `/`。
///
/// 直接委托给 `HomePage` 并固定页码为 1。
#[component]
pub fn Home() -> Element {
    rsx! { HomePage { page: 1 } }
}

/// 首页分页组件，对应路由 `/page/:page`。
///
/// 对传入的页码进行下限校正后，渲染头部信息与文章列表。
#[component]
pub fn HomePage(page: i32) -> Element {
    let current_page = page.max(1);

    rsx! {
        HomeInfo {}
        HomePosts { current_page }
    }
}

/// 首页文章列表与分页展示组件。
///
/// 通过 `use_server_future` 异步获取当前页文章；
/// 加载中显示骨架屏，加载失败显示错误提示，成功则渲染文章卡片与分页。
#[component]
fn HomePosts(current_page: i32) -> Element {
    // 调用 server function 获取已发布文章分页数据。
    let posts_res = use_server_future(move || list_published_posts(current_page, POSTS_PER_PAGE))?;

    // 将结果映射为更便于本地使用的 (posts, total) 形式。
    let posts_data = posts_res.read().as_ref().map(|r| match r {
        Ok(PostListResponse { posts, total }) => Ok((posts.clone(), *total)),
        Err(e) => Err(e.to_string()),
    });

    match posts_data {
        Some(Ok((posts, total))) => {
            rsx! {
                for post in posts.iter() {
                    PostCard { post: post.clone() }
                }
                // 如果当前页没有任何文章，显示空状态提示。
                if posts.is_empty() {
                    div { class: "text-center text-paper-secondary py-20",
                        "暂无文章"
                    }
                }
                // 在列表底部渲染分页导航。
                Pagination { current_page, total }
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
                DelayedSkeleton { HomeSkeleton {} }
            }
        }
    }
}

/// 首页头部信息组件，展示站点名称与副标题。
#[component]
fn HomeInfo() -> Element {
    rsx! {
        div { class: "mb-10 text-center",
            h1 { class: "text-4xl font-bold leading-tight text-paper-primary tracking-tight",
                "Yggdrasil"
            }
            p { class: "mt-3 text-base text-paper-secondary leading-relaxed",
                "以文字为主的简约博客系统"
            }
        }
    }
}

/// 分页导航组件。
///
/// 根据当前页码与文章总数计算总页数，并渲染上一页/下一页链接。
/// 第一页的上一页链接固定指向 `Route::Home`，避免生成 `/page/1`。
#[component]
fn Pagination(current_page: i32, total: i64) -> Element {
    let has_prev = current_page > 1;
    // 向上取整计算总页数，至少为 1 页。
    let total_pages = ((total + POSTS_PER_PAGE as i64 - 1) / POSTS_PER_PAGE as i64).max(1) as i32;
    let has_next = current_page < total_pages;
    let prev = current_page - 1;
    // 当上一页为第 1 页时，使用 `/` 路由而非 `/page/1`。
    let prev_route = if prev <= 1 {
        Route::Home {}
    } else {
        Route::HomePage { page: prev }
    };

    rsx! {
        nav { class: "flex mt-10 mb-6 justify-between",
            if has_prev {
                Link {
                    class: "inline-flex items-center px-4 py-2 text-sm text-white bg-paper-accent rounded-full hover:brightness-110 active:scale-[0.98] transition-all duration-200 cursor-pointer",
                    to: prev_route,
                    span { class: "mr-1", "«" }
                    "上一页"
                }
            }
            if has_next {
                Link {
                    class: "ml-auto inline-flex items-center px-4 py-2 text-sm text-white bg-paper-accent rounded-full hover:brightness-110 active:scale-[0.98] transition-all duration-200 cursor-pointer",
                    to: Route::HomePage { page: current_page + 1 },
                    "下一页"
                    span { class: "ml-1", "»" }
                }
            }
        }
    }
}

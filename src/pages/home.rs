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

use crate::api::posts::{list_published_posts, PostListResponse};
use crate::components::post_card::PostCard;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::home_skeleton::HomeSkeleton;
use crate::components::ui::Pagination;
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
                // total == 0 表示站点确实无文章：显示空状态，且不渲染分页。
                // 注意：total > 0 但 posts 为空（如越界页码 /page/9999）也不显示空状态，
                // 避免误导用户以为站点无内容；此时仅靠下方的分页导航引导回有效页。
                if total == 0 {
                    div { class: "text-center text-paper-secondary py-20",
                        "暂无文章"
                    }
                }
                // 仅在有文章时渲染分页导航，避免越界页码下出现孤立的空分页。
                // frontend variant 不渲染页码计数，unit 不显示（仅满足必填 prop）。
                if total > 0 {
                    Pagination {
                        variant: "frontend",
                        current_page,
                        total,
                        per_page: POSTS_PER_PAGE,
                        prev_route: if current_page - 1 <= 1 {
                            Route::Home {}
                        } else {
                            Route::HomePage { page: current_page - 1 }
                        },
                        next_route: Route::HomePage { page: current_page + 1 },
                        unit: "篇",
                    }
                }
            }
        }
        // 不透传内部错误细节，统一展示通用文案（与标签页等其它页面一致）。
        Some(Err(_)) => {
            rsx! {
                div { class: "text-center text-red-500 dark:text-red-400 py-20",
                    "加载失败"
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


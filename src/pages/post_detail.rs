//! 文章详情页面模块。
//!
//! 对应路由 `/post/:slug`。
//!
//! 数据获取：通过 `use_server_future` 调用 `get_post_by_slug` server function，
//! 根据 URL 中的 slug 获取单篇文章详情（含正文 HTML、目录、封面及上下篇导航）。
//!
//! # 反应式取数的关键
//! Dioxus 0.7 的 `use_server_future`（内部即 `use_resource`）只在闭包内读取的
//! **signal** 变化时才会重跑 future——它通过 `ReactiveContext` 追踪闭包执行期间的
//! 订阅。但本组件的 `slug` 是路由宏注入的普通 `String` prop，被 `move` 进闭包后
//! 成了冻结快照，读取它不会建立订阅。因此上/下一篇导航（同一路由变体间的 slug
//! 变化）会复用组件实例、更新 props，却无法触发 future 重跑——表现为「URL 变了
//! 但内容不变，刷新才生效」。
//!
//! 修复：在闭包内通过 `router().current::<Route>()` 读取当前 slug。`current()`
//! 内部调用 `subscribe_to_current_context()`，在 `use_server_future` 的
//! ReactiveContext 中注册订阅；路由变化时订阅触发，future 自动重跑。
//! 在 `wasm32` 目标下，server function 的函数体被替换为向服务端端点发起 HTTP POST 请求的客户端存根；
//! 实际的数据库访问逻辑仅在 `feature = "server"` 启用时运行。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::api::posts::{get_post_by_slug, SinglePostResponse};
use crate::components::post::post_content::PostContent;
use crate::components::post::post_cover::PostCover;
use crate::components::post::post_footer::PostFooter;
use crate::components::post::post_header::PostHeader;
use crate::components::post::post_toc::PostToc;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::post_detail_skeleton::PostDetailSkeleton;
use crate::router::Route;

/// 文章详情页面组件，对应路由 `/post/:slug`。
///
/// 根据 slug 异步获取文章，渲染文章头部、封面、目录、正文、页脚及评论区；
/// 若文章不存在或加载失败，则展示对应的提示页面。
#[component]
pub fn PostDetail(slug: String) -> Element {
    // 取得路由上下文句柄（不订阅组件层渲染，仅在闭包内按需订阅）。
    // 见模块文档：必须在闭包内读取路由状态才能建立反应式订阅，future 才会在
    // slug 变化（上/下一篇导航）时重跑。`slug` prop 本身是冻结的 String 快照，
    // 不能作为依赖。
    let router = dioxus::router::router();

    let post = use_server_future(move || {
        // 在闭包内读取当前 slug：current() 内部会 subscribe_to_current_context()，
        // 把订阅注册到 use_server_future 的 ReactiveContext，路由变化即重跑。
        let current_slug = match router.current::<Route>() {
            Route::PostDetail { slug } => slug,
            // 组件卸载/路由切走的瞬间可能命中其它变体，退回用 prop 值兜底。
            _ => slug.clone(),
        };
        get_post_by_slug(current_slug)
    })?;

    // 将结果映射为更直观的 Ok(post) / Err("not_found") / Err("error") 三种状态。
    let post_data = post.read().as_ref().map(|r| match r {
        Ok(SinglePostResponse { post: Some(post) }) => Ok(post.clone()),
        Ok(SinglePostResponse { post: None }) => Err("not_found"),
        Err(_) => Err("error"),
    });

    match post_data {
        Some(Ok(post)) => {
            rsx! {
                article { class: "post-single animate-page-enter", key: "{post.slug}",
                    PostHeader { post: post.clone() }

                    // 如果文章设置了封面图，则渲染封面组件。
                    if let Some(cover) = &post.cover_image {
                        PostCover { src: cover.clone() }
                    }

                    // 如果文章生成了目录 HTML，则渲染目录组件。
                    if let Some(toc) = &post.toc_html {
                        PostToc { toc_html: toc.clone() }
                    }

                    PostContent { content_html: post.content_html.clone().unwrap_or_default() }

                    PostFooter { post: post.clone() }

                    // 仅对已发布文章展示评论区域，使用 SuspenseBoundary 处理加载状态。
                    if post.status == crate::models::post::PostStatus::Published {
                        div { class: "mt-12 border-t border-gray-200 dark:border-gray-700 pt-8",
                            SuspenseBoundary {
                                fallback: move |_| rsx! {
                                    DelayedSkeleton {
                                        crate::components::skeletons::comment_skeleton::CommentListSkeleton {}
                                    }
                                },
                                crate::components::comments::section::CommentSection { post_id: post.id }
                            }
                        }
                    }
                }
            }
        }
        Some(Err("not_found")) => {
            rsx! {
                div { class: "text-center py-20",
                    h2 { class: "text-2xl font-bold text-paper-primary mb-4", "文章不存在" }
                    p { class: "text-paper-secondary mb-6",
                        "这篇文章可能已被删除或移动。"
                    }
                    Link {
                        class: "px-6 py-2 bg-paper-primary text-paper-theme rounded-full font-medium hover:opacity-80 transition-opacity",
                        to: Route::Home {},
                        "返回首页"
                    }
                }
            }
        }
        Some(Err("error")) => {
            rsx! {
                div { class: "text-center text-red-500 dark:text-red-400 py-20", "加载失败" }
            }
        }
        _ => {
            rsx! {
                DelayedSkeleton { PostDetailSkeleton {} }
            }
        }
    }
}

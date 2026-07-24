//! 404 页面模块。
//!
//! 对应路由 `/:..segments`。
//!
//! 当用户访问未匹配任何前端路由的 URL 时，Dioxus Router 会回退到该 404 页面。
//! 该页面为静态展示页面，不发起任何 server function 调用。

use dioxus::prelude::*;

use crate::router::Route;

/// 404 页面组件，对应兜底路由 `/:..segments`。
///
/// 展示大号的装饰性 404 数字、状态标签、错误说明以及返回首页的链接。
///
/// # 两种命中路径
/// 本组件在两种完全不同的机制下被渲染：
/// 1. **路由匹配** —— 访问任意未命中路径，Router 命中 catch-all `Route::NotFound`，
///    此时 `ErrorBoundary` **无错误**，本组件作为 children（Outlet）正常渲染。
/// 2. **错误冒泡** —— 如 `PostDetail` 对不存在的 slug 抛出 `ServerFnError(404)`，
///    `ErrorLayout` 的 `ErrorBoundary` 捕获后在 fallback 里渲染本组件。
///
/// 「返回首页」必须在导航前清除可能存在的错误边界，否则场景 2 会卡死：
/// ErrorBoundary 持有错误时不渲染 children（Outlet），路由虽切到 `Home`，
/// 页面仍停留在 fallback（本组件），表现为「URL 变了但页面不变」。
/// 场景 1 下没有错误，`clear_errors` 是 no-op。
#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let _ = segments;

    // Commit 404 status code during server-side rendering
    #[cfg(feature = "server")]
    {
        dioxus::fullstack::FullstackContext::commit_http_status(
            http::StatusCode::NOT_FOUND,
            Some("Page Not Found".to_string()),
        );
    }

    rsx! {
        div { class: "flex flex-col items-center justify-center text-center min-h-[50vh] md:min-h-[55vh] px-6",
            // 巨大的装饰性 404，作为视觉锚点
            div { class: "relative mb-2",
                span { class: "text-[140px] md:text-[180px] font-bold leading-none tracking-tighter text-paper-tertiary/[0.08] dark:text-paper-tertiary/[0.06] select-none",
                    "404"
                }
            }

            // 状态标签
            span { class: "text-sm font-medium tracking-[0.2em] uppercase text-paper-secondary mb-6",
                "Page Not Found"
            }

            // 分隔线
            div { class: "w-12 h-px bg-paper-border mb-8" }

            // 错误信息
            h1 { class: "text-xl md:text-2xl font-medium text-paper-primary mb-3",
                "页面未找到"
            }
            p { class: "text-sm md:text-base text-paper-secondary max-w-sm leading-relaxed mb-10",
                "这个页面似乎走丢了，或者从未存在过。"
            }

            // 返回首页：用 onclick 先清除错误边界再导航。
            // 直接用 Link 无法干预点击时机，故改为按钮 + 命令式导航。
            // 详见组件顶部文档：场景 2（错误冒泡）下若不清除错误，
            // ErrorBoundary 会一直渲染 fallback，路由切换后页面仍卡在本页。
            button {
                r#type: "button",
                onclick: move |_| {
                    if let Some(ctx) = try_consume_context::<ErrorContext>() {
                        ctx.clear_errors();
                    }
                    let _ = dioxus::router::navigator().push(Route::Home {});
                },
                class: "group inline-flex items-center gap-2 px-5 py-2.5 text-sm font-medium text-paper-primary bg-paper-entry border border-paper-border rounded-lg hover:border-paper-secondary hover:bg-paper-border transition-all cursor-pointer",
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    width: "16",
                    height: "16",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    class: "transition-transform group-hover:-translate-x-0.5",
                    path { d: "M19 12H5M12 19l-7-7 7-7" }
                }
                "返回首页"
            }
        }
    }
}

//! 404 页面模块。
//!
//! 对应路由 `/:..segments`。
//!
//! 当用户访问未匹配任何前端路由的 URL 时，Dioxus Router 会回退到该 404 页面。
//! 该页面为静态展示页面，不发起任何 server function 调用。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::router::Route;

/// 404 页面组件，对应兜底路由 `/:..segments`。
///
/// 展示大号的装饰性 404 数字、状态标签、错误说明以及返回首页的链接。
#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let _ = segments;

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

            // 返回首页
            Link {
                to: Route::Home {},
                class: "group inline-flex items-center gap-2 px-5 py-2.5 text-sm font-medium text-paper-primary bg-paper-entry border border-paper-border rounded-lg hover:border-paper-secondary hover:bg-paper-border transition-all",
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

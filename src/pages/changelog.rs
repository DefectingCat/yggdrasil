//! 更新日志页面模块。
//!
//! 对应路由 `/changelog`。
//!
//! 数据获取：`use_server_future` 调用 `get_changelog` server function，取回
//! 编译期内嵌 CHANGELOG.md 的服务端渲染结果（正文 HTML + 版本索引 TOC）。
//! 内容随二进制版本固定，只受 SSR 页面缓存 TTL 约束，无需任何主动失效。
//! 页面无路由参数、future 不会重跑，因此无需 `router().current()` 订阅
//! （该陷阱详见 `post_detail.rs` 头文档）。

use dioxus::prelude::*;

use crate::api::changelog::{get_changelog, ChangelogResponse};
use crate::components::post::post_toc::PostToc;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::post_detail_skeleton::PostDetailSkeleton;

/// 更新日志页面组件，对应路由 `/changelog`。
///
/// 结构：页头 → 版本索引（可折叠 TOC）→ 按版本分段的全文。
/// 正文复用 `md-content` 排版（与文章详情同一套样式），不新增 CSS。
#[component]
pub fn Changelog() -> Element {
    let response = use_server_future(get_changelog)?;

    // 与 post_detail 同一约定：None（加载中）→ 骨架屏；Err → 抛给错误边界。
    let data = response.read().as_ref().map(|r| match r {
        Ok(resp) => Ok(resp.clone()),
        Err(e) => Err(e.clone()),
    });

    let ChangelogResponse { html, toc_html } = match data {
        Some(Ok(resp)) => resp,
        Some(Err(err)) => return Err(err.into()),
        None => {
            return rsx! {
                DelayedSkeleton { PostDetailSkeleton {} }
            };
        }
    };

    rsx! {
        div { class: "animate-page-enter",
            header { class: "page-header mb-6",
                h1 { class: "text-4xl font-bold text-paper-primary tracking-tight",
                    "更新日志"
                }
            }

            if !toc_html.is_empty() {
                PostToc { toc_html, title: "版本索引" }
            }

            div {
                class: "post-content md-content",
                dangerous_inner_html: "{html}",
            }
        }
    }
}

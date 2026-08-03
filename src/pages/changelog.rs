//! 更新日志页面模块。
//!
//! 对应路由 `/changelog`。
//!
//! 数据获取：`use_server_future` 调用 `get_changelog` server function，取回
//! 编译期内嵌 CHANGELOG.md 的**结构化解析结果**（版本 → 分类 → 条目 HTML）。
//! 内容随二进制版本固定，只受 SSR 页面缓存 TTL 约束，无需任何主动失效。
//! 页面无路由参数、future 不会重跑，因此无需 `router().current()` 订阅
//! （该陷阱详见 `post_detail.rs` 头文档）。
//!
//! # 布局
//! 双栏：左侧 sticky 版本导航（桌面端）+ 右侧版本卡片列表。
//! 每个版本卡片内按分类（Added / Fixed / Security …）分组，每组带色标 badge。
//! 配色遵循全站 Catppuccin 双强调色约束（详见 `changelog.rs` 模块文档）。

use dioxus::prelude::*;

use crate::api::changelog::{get_changelog, ChangeGroup, ChangelogData, VersionEntry};
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::post_detail_skeleton::PostDetailSkeleton;

/// 更新日志页面组件，对应路由 `/changelog`。
///
/// 结构：页头 → 统计栏 → 双栏（版本导航 + 版本卡片列表）。
#[component]
pub fn Changelog() -> Element {
    let response = use_server_future(get_changelog)?;

    // 与 post_detail 同一约定：None（加载中）→ 骨架屏；Err → 抛给错误边界。
    let data = response.read().as_ref().map(|r| match r {
        Ok(resp) => Ok(resp.clone()),
        Err(e) => Err(e.clone()),
    });

    let ChangelogData { versions } = match data {
        Some(Ok(resp)) => resp,
        Some(Err(err)) => return Err(err.into()),
        None => {
            return rsx! {
                DelayedSkeleton { PostDetailSkeleton {} }
            };
        }
    };

    let total = versions
        .iter()
        .filter(|v| v.version != "Unreleased")
        .count();
    let latest = versions.iter().find(|v| v.is_latest);

    rsx! {
        div { class: "animate-page-enter",
            header { class: "page-header mb-6",
                h1 { class: "text-4xl font-bold text-paper-primary tracking-tight",
                    "更新日志"
                }
            }

            // 统计栏：最新版本 + 版本总数。镜像 post-meta 的安静感。
            div { class: "flex flex-wrap items-center gap-x-4 gap-y-1 mb-8 text-sm text-paper-tertiary",
                if let Some(v) = latest {
                    div { class: "flex items-center gap-1.5",
                        span { class: "w-2 h-2 rounded-full bg-[var(--color-paper-accent)]" }
                        span { "最新版本 " }
                        span { class: "font-medium text-paper-primary", "v{v.version}" }
                    }
                }
                span { "共 {total} 个版本" }
            }

            // 双栏布局
            div { class: "flex gap-8",
                // 版本导航：桌面端 sticky 侧栏
                if versions.len() > 1 {
                    nav { class: "hidden lg:block w-40 shrink-0",
                        div { class: "sticky top-20",
                            for v in versions.iter() {
                                VersionNavItem {
                                    key: "{v.version}",
                                    version: v.version.clone(),
                                    is_latest: v.is_latest,
                                }
                            }
                        }
                    }
                }

                // 版本卡片列表
                div { class: "flex-1 min-w-0 space-y-6",
                    for v in versions.iter() {
                        VersionCard { key: "{v.version}", version: v.clone() }
                    }
                }
            }
        }
    }
}

/// 版本导航侧栏中的单条链接。
#[component]
fn VersionNavItem(version: String, is_latest: bool) -> Element {
    let base = "group flex items-center gap-2 py-1.5 text-sm transition-colors";
    let text_class = if is_latest {
        "font-medium text-paper-accent"
    } else {
        "text-paper-secondary group-hover:text-paper-primary"
    };
    let dot_class = if is_latest {
        "bg-[var(--color-paper-accent)]"
    } else {
        "bg-[var(--color-paper-border)]"
    };

    rsx! {
        a {
            href: "#v{version}",
            class: "{base}",
            span { class: "w-1.5 h-1.5 rounded-full shrink-0 {dot_class}" }
            span { class: "{text_class}", "{version}" }
        }
    }
}

/// 单个版本卡片。
///
/// 结构：版本头（版本号 + 最新标记 + 日期）→ intro（如有）→ 分类组列表。
/// 每个分类组带色标 badge + 条目列表。
#[component]
fn VersionCard(version: VersionEntry) -> Element {
    let VersionEntry {
        version: ver,
        date,
        is_latest,
        intro_html,
        groups,
    } = version;

    rsx! {
        article {
            id: "v{ver}",
            class: "changelog-version scroll-mt-20 rounded-[2rem] bg-[var(--color-paper-entry)] border border-transparent hover:border-[var(--color-paper-border)] transition-colors p-6 md:p-8",

            // 版本头
            div { class: "flex items-baseline gap-3",
                h2 { class: "text-xl md:text-2xl font-bold text-paper-primary tracking-tight",
                    "v{ver}"
                }
                if is_latest {
                    span { class: "changelog-badge changelog-badge--latest", "最新" }
                }
                if let Some(d) = &date {
                    span { class: "text-sm text-paper-tertiary ml-auto", "{d}" }
                }
            }

            // intro（Unreleased 占位文字等）
            if !intro_html.is_empty() {
                div {
                    class: "md-content text-sm text-paper-secondary mt-2",
                    dangerous_inner_html: "{intro_html}",
                }
            }

            // 分类组
            if !groups.is_empty() {
                div { class: "mt-4 space-y-5",
                    for group in groups.iter() {
                        ChangeGroupView { key: "{group.category:?}", group: group.clone() }
                    }
                }
            }
        }
    }
}

/// 单个分类组视图：badge + 条目列表。
#[component]
fn ChangeGroupView(group: ChangeGroup) -> Element {
    let ChangeGroup { category, items_html } = group;
    let badge_class = format!("changelog-badge changelog-badge--{}", category.css_class());

    rsx! {
        div {
            // badge 行
            div { class: "mb-2",
                span { class: "{badge_class}", "{category.label()}" }
            }
            // 条目列表（复用 md-content 内联格式 + changelog-items 列表样式覆盖）
            div {
                class: "md-content changelog-items",
                dangerous_inner_html: "{items_html}",
            }
        }
    }
}

//! 文章管理页面（列表 + 回收站，单一路由 + 客户端 tab 切换）。
//!
//! 「全部文章」与「回收站」合并为单一 `/admin/posts` 路由，用顶部 tab 在二者间
//! 切换。tab 状态与翻页均由客户端 signal 驱动（不走路由、不深链），与 `system.rs`
//! 的 tab 模式一致：admin 内部页，刷新回到「全部文章」第 1 页即可。
//! 数据加载与写操作仅在 WASM 前端通过 Dioxus server functions 完成。

use dioxus::prelude::*;
use dioxus::router::components::Link;

// 分页数据接口：list_posts 是 server function，两端都生成（wasm 端为 client stub，
// server 端为真实实现），故无需 cfg。实际请求只在 use_paginated 的 wasm 分支发出。
use crate::api::posts::{list_posts, PostListResponse};
// get_post_stats / PostStatsResponse 仅在 Posts 容器的 wasm 加载路径使用，
// SSR 下对应 use_effect 分支被裁剪，故允许 unused imports。
#[allow(unused_imports)]
use crate::api::posts::{
    delete_post, get_post_stats, rebuild_content_html, rebuild_post_content_html,
    CreatePostResponse, PostStatsResponse, RebuildResult,
};
use crate::components::empty_state::{EmptyState, EmptyStateAction};
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::skeletons::posts_skeleton::PostsSkeleton;
use crate::components::ui::{
    Pagination, StatusBadge, Tooltip, ADMIN_ROW_HOVER, ADMIN_TABLE_CLASS, BTN_OUTLINE, BTN_PRIMARY,
    BTN_TEXT_ACCENT, BTN_TEXT_RED, SPINNER_SVG,
};
use crate::hooks::query::use_paginated;
use crate::models::post::PostListItem;
use crate::router::Route;
// 回收站 tab 内容（本容器 match 渲染）。
use super::posts_trash::PostsTrashPanel;

/// 每页展示的文章数量。
const POSTS_PER_PAGE: i32 = 20;

/// 文章管理顶部 tab：全部文章 / 回收站。
///
/// 用枚举而非裸字符串，保证 tab 切换的类型安全；`as_str()` 提供稳定 key 供
/// `key` 化重挂载（隔离各 tab 的 `use_paginated` 状态）。
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum PostsTab {
    /// 全部文章（含草稿）。
    All,
    /// 回收站（已软删除）。
    Trash,
}

impl PostsTab {
    fn as_str(&self) -> &'static str {
        match self {
            PostsTab::All => "all",
            PostsTab::Trash => "trash",
        }
    }
}

/// 文章管理入口组件：单一路由 + 客户端 tab 切换。
///
/// 持有 `active_tab` signal 与回收站数量 `trash_count`（供 header 文案 + tab 角标），
/// 用 `key` 化 `match` 切换 `AllPostsList` / `PostsTrashPanel`：切 tab 完全卸载旧
/// 组件、重挂新组件，各 tab 的 `use_paginated` / 选中态等本地 signal 天然隔离，
/// 无需手动重置。参照 `system.rs` 的 tab 模式。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
pub fn Posts() -> Element {
    let mut active_tab = use_signal(|| PostsTab::All);
    // 回收站数量：仅 WASM 异步拉取一次，供 header 文案「已删除文章 (N)」与 tab 角标。
    // 回收站 panel 内部维护自己的精确 total（分页计数用），二者解耦——角标是粗略提示。
    let mut trash_count = use_signal(|| Option::<i64>::None);

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        spawn(async move {
            if let Ok(PostStatsResponse { stats }) = get_post_stats().await {
                trash_count.set(Some(stats.trash));
            }
        });
    });

    rsx! {
        div { class: "w-full max-w-7xl mx-auto space-y-6",
            // 共享 header：标题/副标题随 tab 切换文案；右侧操作区仅「全部文章」tab 显示。
            div { class: "flex flex-col md:flex-row md:items-end justify-between gap-6 pb-6 border-b border-paper-border mb-6",
                div {
                    h1 { class: "text-4xl font-extrabold tracking-tight text-[var(--color-paper-primary)]",
                        if active_tab() == PostsTab::All {
                            "管理文章"
                        } else {
                            "回收站"
                        }
                    }
                    p { class: "text-base text-[var(--color-paper-secondary)] mt-2",
                        if active_tab() == PostsTab::All {
                            "所有文章及草稿"
                        } else {
                            if let Some(count) = trash_count() {
                                "已删除文章 ({count})"
                            } else {
                                "已删除文章"
                            }
                        }
                    }
                }
                // 发布文章 + 重建缓存仅在「全部文章」tab 显示。
                if active_tab() == PostsTab::All {
                    div { class: "flex items-center gap-3",
                        RebuildCacheBar {}
                        Link { class: "{BTN_PRIMARY}", to: Route::Write {}, "发布文章" }
                    }
                }
            }

            // tab 栏：全部文章 / 回收站。signal 驱动（点击切 active_tab，非路由）。
            PostsTabs {
                active: active_tab,
                trash_count,
                on_change: move |t: PostsTab| active_tab.set(t),
            }

            // key 化条件渲染：切 tab 完全卸载/重挂，隔离各自 use_paginated 状态。
            div { key: "{active_tab().as_str()}",
                match active_tab() {
                    PostsTab::All => rsx! {
                        AllPostsList {}
                    },
                    PostsTab::Trash => rsx! {
                        PostsTrashPanel {}
                    },
                }
            }
        }
    }
}

/// 全部文章列表 tab：分页列表、删除单篇、重建 content_html 缓存。
///
/// 翻页用客户端 signal 驱动（`current_page` signal + `use_paginated` 的闭包内读取
/// 建立依赖，页码变化自动重载），不走路由。删除/重建逻辑与旧实现一致。
#[component]
fn AllPostsList() -> Element {
    let current_page = use_signal(|| 1);

    // 分页列表加载（loading / posts / total / error）由 use_paginated 统一管理。
    // 闭包内读取 current_page（.with）建立 reactive 依赖，翻页时自动重新请求。
    let paginated = use_paginated(
        move || current_page.with(|p| *p),
        POSTS_PER_PAGE,
        |p, pp| async move {
            list_posts(p, pp)
                .await
                .map(|PostListResponse { posts, total }| (posts, total))
        },
    );
    let mut posts = paginated.items;
    let mut total = paginated.total;
    let loading = paginated.loading;
    let _error = paginated.error;

    // 删除中 / 重建中文章 ID 集合：均由本组件持有（业务逻辑不归 hook 管）。
    // 改为非乐观删除后行会保留至请求完成，可并发点多个删除，故用 HashSet
    // 与 rebuilding 同形，按行通过 contains 判断 loading 态。
    let mut deleting = use_signal(std::collections::HashSet::<i32>::new);
    // 重建中文章 ID 集合：支持多篇文章并发重建（行不会随点击消失，单值会被后点
    // 的覆盖先点的，故用 HashSet），按行通过 contains 判断 loading 态。
    let mut rebuilding = use_signal(std::collections::HashSet::<i32>::new);
    let get_posts = move || -> Vec<PostListItem> { posts() };

    rsx! {
        if loading() && posts().is_empty() {
            DelayedSkeleton { PostsSkeleton {} }
        } else if posts().is_empty() {
            EmptyState {
                title: "暂无文章",
                description: "还没有创建任何文章，开始写下你的第一篇文字吧。",
                action: EmptyStateAction {
                    label: "写文章".to_string(),
                    to: Route::Write {},
                },
            }
        } else {
            div { class: "{ADMIN_TABLE_CLASS}",
                table { class: "w-full text-sm",
                    thead {
                        tr { class: "border-b border-paper-border text-left text-paper-secondary",
                            th { class: "px-4 py-3 font-medium", "标题" }
                            th { class: "px-4 py-3 font-medium w-24 text-center", "状态" }
                            th { class: "px-4 py-3 font-medium w-32", "日期" }
                            th { class: "px-4 py-3 font-medium w-44 text-right", "操作" }
                        }
                    }
                    tbody {
                        for post in get_posts().iter() {
                            PostRow {
                                key: "{post.id}",
                                post: post.clone(),
                                deleting: deleting().contains(&post.id),
                                rebuilding: rebuilding().contains(&post.id),
                                on_delete: move |id| {
                                    deleting.write().insert(id);
                                    spawn(async move {
                                        match delete_post(id).await {
                                            Ok(CreatePostResponse { success: true, .. }) => {
                                                posts.with_mut(|list| list.retain(|p| p.id != id));
                                                total.with_mut(|t| *t = t.saturating_sub(1));
                                            }
                                            Ok(CreatePostResponse { success: false, message: _message, .. }) => {
                                                #[cfg(target_arch = "wasm32")]
                                                web_sys::window().map(|w| w.alert_with_message(&_message).ok());
                                            }
                                            Err(_e) => {
                                                #[cfg(target_arch = "wasm32")]
                                                web_sys::window().map(|w| w.alert_with_message("删除失败").ok());
                                            }
                                        }
                                        deleting.write().remove(&id);
                                    });
                                },
                                on_rebuild: move |id| {
                                    rebuilding.write().insert(id);
                                    spawn(async move {
                                        let _ = rebuild_post_content_html(id).await;
                                        rebuilding.write().remove(&id);
                                    });
                                },
                            }
                        }
                    }
                }
            }
            Pagination {
                variant: "admin",
                current_page: current_page(),
                total: total(),
                per_page: POSTS_PER_PAGE,
                unit: "篇",
                on_prev: {
                    let mut page = current_page;
                    move |_| {
                        page.with_mut(|p| *p = (*p - 1).max(1));
                    }
                },
                on_next: {
                    let mut page = current_page;
                    move |_| {
                        page.with_mut(|p| *p += 1);
                    }
                },
            }
        }
    }
}

/// 重建内容缓存工具条子组件。
///
/// 封装「重建内容 / 重建全部」两个按钮及其 `do_rebuild` 异步闭包。状态
/// (`rebuilding` / `rebuild_result`) 由本组件内部持有（从 `PostsPage` 下沉至此，
/// 因合并后仅 All tab 需要，无需跨层传递）。
///
/// 从 `AllPostsList` 抽取以降低 god component 复杂度（见 dioxus-render-purity skill）。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
fn RebuildCacheBar() -> Element {
    let mut rebuilding = use_signal(|| false);
    let mut rebuild_result = use_signal(|| Option::<String>::None);

    // 重建文章渲染缓存：rebuild_all 为 false 时仅重建 content_html 为空的文章，
    // 为 true 时重建所有文章（用于语法/渲染逻辑升级后批量刷新已有内容）。
    let mut do_rebuild = move |rebuild_all: bool| {
        rebuilding.set(true);
        rebuild_result.set(None);
        spawn(async move {
            match rebuild_content_html(rebuild_all).await {
                Ok(RebuildResult {
                    rebuilt,
                    failed,
                    errors,
                }) => {
                    if failed > 0 {
                        let mut msg = format!("已重建 {rebuilt} 篇，失败 {failed} 篇");
                        if let Some(first) = errors.first() {
                            msg.push_str(&format!("\n{first}"));
                        }
                        rebuild_result.set(Some(msg));
                    } else {
                        rebuild_result.set(Some(format!("已重建 {rebuilt} 篇文章")));
                    }
                }
                Err(e) => {
                    rebuild_result.set(Some(format!("失败: {e}")));
                }
            }
            rebuilding.set(false);
        });
    };

    rsx! {
        // 消息绝对定位到按钮行下方，脱离文档流：出现/消失都不撑高祖先容器，
        // 避免 header 的 md:items-end 把固定底边转化为按钮上移（"按钮被顶上去" bug）。
        // 自持 rebuilding / rebuild_result state，与父组件零耦合。
        div { class: "relative flex items-center gap-3",
            div { class: "flex items-center gap-3",
                Tooltip {
                    tip: "重建 content_html 为空的文章渲染缓存".to_string(),
                    placement: "bottom",
                    button {
                        class: if rebuilding() { "relative px-4 py-2 rounded-full text-sm font-medium cursor-not-allowed text-paper-secondary border border-paper-border" } else { BTN_OUTLINE },
                        disabled: rebuilding(),
                        onclick: move |_| do_rebuild(false),
                        span { class: if rebuilding() { "opacity-40" } else { "" }, "重建内容" }
                        if rebuilding() {
                            span {
                                class: "absolute inset-0 flex items-center justify-center",
                                dangerous_inner_html: SPINNER_SVG,
                            }
                        }
                    }
                }
                Tooltip {
                    tip: "重建所有文章的渲染缓存（含已有内容）".to_string(),
                    placement: "bottom",
                    button {
                        class: if rebuilding() { "relative px-4 py-2 rounded-full text-sm font-medium cursor-not-allowed text-paper-secondary border border-paper-border" } else { BTN_OUTLINE },
                        disabled: rebuilding(),
                        onclick: move |_| do_rebuild(true),
                        span { class: if rebuilding() { "opacity-40" } else { "" }, "重建全部" }
                        if rebuilding() {
                            span {
                                class: "absolute inset-0 flex items-center justify-center",
                                dangerous_inner_html: SPINNER_SVG,
                            }
                        }
                    }
                }
            }
            // 重建结果消息：绝对定位到按钮行正下方，脱离文档流，不影响布局高度。
            if let Some(msg) = rebuild_result() {
                div { class: "absolute top-full right-0 mt-1 text-xs text-paper-secondary whitespace-pre-line",
                    "{msg}"
                }
            }
        }
    }
}

/// 文章表格行组件，展示单篇文章的标题、状态、日期与操作按钮。
#[component]
fn PostRow(
    post: PostListItem,
    deleting: bool,
    rebuilding: bool,
    on_delete: EventHandler<i32>,
    on_rebuild: EventHandler<i32>,
) -> Element {
    let date_str = post.formatted_date();

    rsx! {
        tr { class: "{ADMIN_ROW_HOVER}",
            td { class: "px-4 py-3",
                Link {
                    class: "text-paper-primary hover:text-paper-accent transition-colors cursor-pointer",
                    to: Route::PostDetail {
                        slug: post.slug.clone(),
                    },
                    "{post.title}"
                }
            }
            td { class: "px-4 py-3 text-center",
                StatusBadge {
                    color_class: post.status_badge_class(),
                    label: post.status_label().to_string(),
                }
            }
            td { class: "px-4 py-3 text-paper-secondary", "{date_str}" }
            td { class: "px-4 py-3 text-right",
                div { class: "flex justify-end items-center gap-3",
                    Link {
                        class: "text-xs text-paper-secondary hover:text-paper-primary transition-colors cursor-pointer",
                        to: Route::WriteEdit { id: post.id },
                        "编辑"
                    }
                    Tooltip { tip: "重新渲染这篇文章的 HTML".to_string(),
                        button {
                            class: if rebuilding { "relative inline-flex items-center text-xs text-paper-accent cursor-not-allowed" } else { BTN_TEXT_ACCENT },
                            disabled: rebuilding,
                            onclick: move |_| on_rebuild.call(post.id),
                            span { class: if rebuilding { "opacity-40" } else { "" }, "重建" }
                            if rebuilding {
                                span {
                                    class: "absolute inset-0 flex items-center justify-center",
                                    dangerous_inner_html: SPINNER_SVG,
                                }
                            }
                        }
                    }
                    button {
                        class: if deleting { "relative inline-flex items-center text-xs text-paper-secondary cursor-not-allowed" } else { BTN_TEXT_RED },
                        disabled: deleting,
                        onclick: move |_| on_delete.call(post.id),
                        span { class: if deleting { "opacity-40" } else { "" }, "删除" }
                        if deleting {
                            span {
                                class: "absolute inset-0 flex items-center justify-center",
                                dangerous_inner_html: SPINNER_SVG,
                            }
                        }
                    }
                }
            }
        }
    }
}

/// tab 组 id 自增计数器：给每处 PostsTabs 实例一个唯一前缀，用于 DOM 测量滑块位置。
/// （ui.rs 的 FilterTabs 有同款 TAB_GROUP_ID，此处为避免跨模块可见性污染，本地自建。）
static POSTS_TAB_GROUP_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// 文章管理 tab 栏：「全部文章」与「回收站」。
///
/// tab 状态由父组件传入的 `active` signal 驱动（非路由），点击即调用 `on_change`
/// 切换。回收站 tab 带 `trash_count` 数量角标，便于发现待清理文章。
///
/// 底部带**平滑滑动指示器**（绝对定位的滑块 + transition），切换 tab 时滑块从
/// 一个 tab 平滑滑到另一个，与 FilterTabs（system/comments 页）视觉一致。滑块
/// 位置通过 WASM 端测量目标 button 的 offsetLeft/offsetWidth 动态计算。
///
/// 两个 tab 均用 `inline-flex items-center` 同盒模型，外层容器加 `items-center`，
/// 根除原先「全部文章」(inline 文本) 与「回收站」(inline-flex 带角标) 盒模型
/// 不一致导致的垂直错位。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
pub(super) fn PostsTabs(
    active: Signal<PostsTab>,
    trash_count: Signal<Option<i64>>,
    on_change: EventHandler<PostsTab>,
) -> Element {
    let is_trash = active() == PostsTab::Trash;
    // 滑块样式（left/width/opacity）：WASM 端测量目标 button 定位后写入。
    let mut indicator_style = use_signal(|| "left: 0px; width: 0px; opacity: 0;".to_string());
    let id_prefix =
        use_hook(|| POSTS_TAB_GROUP_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst));

    // 测量目标 tab 的 offsetLeft/offsetWidth，更新滑块定位。WASM 端异步等待 DOM
    // 更新后读取；server 端空操作（SSR 不渲染动画）。
    let update_indicator = move |active_key: &str| {
        let active_key = active_key.to_string();
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                crate::utils::time::sleep_ms(50).await;
                if let Some(el) = web_sys::window().and_then(|w| w.document()).and_then(|d| {
                    d.get_element_by_id(&format!("posts-tab-{id_prefix}-{active_key}"))
                }) {
                    if let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>() {
                        indicator_style.set(format!(
                            "left: {}px; width: {}px; opacity: 1;",
                            html_el.offset_left(),
                            html_el.offset_width()
                        ));
                    }
                }
            }
        });
    };

    // active 变化时（含首次挂载）触发滑块定位。
    use_effect(move || {
        update_indicator(active().as_str());
    });

    rsx! {
        // relative 容器：承载绝对定位滑块；items-center 让两个 tab 垂直居中对齐。
        div { class: "relative flex items-center gap-4 border-b border-paper-border",
            button {
                id: "posts-tab-{id_prefix}-all",
                class: if !is_trash { "inline-flex items-center px-2 py-3 text-xs font-mono tracking-widest uppercase text-paper-primary transition-colors cursor-pointer" } else { "inline-flex items-center px-2 py-3 text-xs font-mono tracking-widest uppercase text-paper-secondary hover:text-paper-primary transition-colors cursor-pointer" },
                onclick: move |_| on_change.call(PostsTab::All),
                "全部文章"
            }
            button {
                id: "posts-tab-{id_prefix}-trash",
                class: if is_trash { "inline-flex items-center gap-1.5 px-2 py-3 text-xs font-mono tracking-widest uppercase text-paper-primary transition-colors cursor-pointer" } else { "inline-flex items-center gap-1.5 px-2 py-3 text-xs font-mono tracking-widest uppercase text-paper-secondary hover:text-paper-primary transition-colors cursor-pointer" },
                onclick: move |_| on_change.call(PostsTab::Trash),
                "回收站"
                // 数量角标：有数据才显示。0 显示中性灰，>0 用主题强调色提醒。
                if let Some(count) = trash_count() {
                    span { class: if count > 0 { "inline-flex items-center justify-center min-w-[1.25rem] h-5 px-1.5 rounded-full text-[0.625rem] font-semibold normal-case tracking-normal bg-paper-accent-soft text-paper-accent" } else { "inline-flex items-center justify-center min-w-[1.25rem] h-5 px-1.5 rounded-full text-[0.625rem] font-semibold normal-case tracking-normal bg-paper-tertiary text-paper-secondary" },
                        "{count}"
                    }
                }
            }
            // 绝对定位的滑动指示器：贴底边（-1px 盖住外层 border-b），transition 驱动滑动动画。
            div {
                class: "absolute bottom-[-1px] h-[2px] bg-paper-primary transition-all duration-300 ease-[cubic-bezier(0.4,0,0.2,1)] pointer-events-none",
                style: "{indicator_style}",
            }
        }
    }
}

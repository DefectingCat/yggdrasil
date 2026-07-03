//! 通用 UI 原子组件与类名常量。
//!
//! 提供跨页面共享的样式常量（卡片、按钮、徽章外层等）与可复用组件
//! （分页导航、状态徽章、空状态）。样式常量用于消除散落在各页面的重复
//! Tailwind 类字符串；组件用于封装结构固定的 UI 单元。
//!
//! 与 `forms.rs`（表单控件）并列，本模块聚焦通用展示类原子。

use dioxus::prelude::*;
use dioxus::router::components::Link;

use crate::router::Route;

// ===========================================================================
// 样式常量
// ===========================================================================

/// Admin 卡片容器：白底圆角描边，亮/暗双模式。用于 stat 卡片、面板等。
pub const ADMIN_CARD_CLASS: &str = "bg-paper-entry rounded-xl border border-paper-border";

/// Admin 表格容器：在卡片基础上加 `overflow-hidden`，圆角裁剪表格。
pub const ADMIN_TABLE_CLASS: &str =
    "bg-paper-entry rounded-xl border border-paper-border overflow-hidden";

/// Admin 表格行 hover 态：底部分割线 + 悬停背景。
pub const ADMIN_ROW_HOVER: &str =
    "border-b border-paper-border last:border-0 hover:bg-paper-entry transition-colors";

/// 列表复选框统一样式（全选表头 + 行内）。
pub const CHECKBOX_CLASS: &str = "rounded border-paper-border";

/// 行内加载 spinner：环形渐变 + 自旋动画，用 currentColor 继承文字色。
///
/// 内联 SVG（含 `@keyframes`），通过 `dangerous_inner_html` 注入；尺寸由外层
/// Tailwind 类（如 `w-3.5 h-3.5`）控制。源文件 `public/icons/90-ring-with-gradient.svg`。
pub const SPINNER_SVG: &str = r#"<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 20 20" xmlns="http://www.w3.org/2000/svg"><defs><linearGradient id="yggSpinnerGrad"><stop offset="0%" stop-color="currentColor" stop-opacity="1"/><stop offset="100%" stop-color="currentColor" stop-opacity="0.25"/></linearGradient></defs><style>@keyframes yggSpin { to { transform: rotate(360deg); } } .ygg-spinner-circle { transform-origin: 50% 50%; stroke: url(#yggSpinnerGrad); fill: none; animation: yggSpin .5s infinite linear; }</style><circle cx="10" cy="10" r="8" class="ygg-spinner-circle" stroke-width="2"/></svg>"#;

/// 状态徽章外层：小号圆角胶囊。
pub const BADGE_BASE: &str =
    "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium whitespace-nowrap";

// --- 实心小按钮（批量操作栏：通过 / 垃圾 / 删除） ---

/// 绿色实心小按钮（批量通过、批量恢复）。
pub const BTN_SOLID_GREEN: &str =
    "px-3 py-1.5 text-xs font-medium bg-green-600 text-white rounded hover:bg-green-700 transition-colors";
/// 琥珀色实心小按钮（批量标为垃圾）。
pub const BTN_SOLID_AMBER: &str =
    "px-3 py-1.5 text-xs font-medium bg-amber-600 text-white rounded hover:bg-amber-700 transition-colors";
/// 红色实心小按钮（批量删除、批量彻底删除）。
pub const BTN_SOLID_RED: &str =
    "px-3 py-1.5 text-xs font-medium bg-red-600 text-white rounded hover:bg-red-700 transition-colors";

// --- 文字小按钮（表格行内操作：通过 / 垃圾 / 删除 / 恢复） ---

/// 绿色文字小按钮（行内通过）。
pub const BTN_TEXT_GREEN: &str = "text-xs text-green-600 hover:text-green-800 dark:text-green-400 dark:hover:text-green-300 transition-colors cursor-pointer";
/// 琥珀色文字小按钮（行内标为垃圾）。
pub const BTN_TEXT_AMBER: &str = "text-xs text-amber-600 hover:text-amber-800 dark:text-amber-400 dark:hover:text-amber-300 transition-colors cursor-pointer";
/// 红色文字小按钮（行内删除 / 彻底删除）。
pub const BTN_TEXT_RED: &str =
    "text-xs text-red-500 hover:text-red-700 dark:hover:text-red-300 transition-colors cursor-pointer";
/// 主题绿（鼠尾草）文字小按钮（行内恢复）。
pub const BTN_TEXT_ACCENT: &str =
    "text-xs text-paper-accent hover:text-paper-primary transition-colors cursor-pointer";

// --- 次要按钮（冷调玫瑰第二色，ghost 描边风格，从属于主色鼠尾草绿） ---

/// 次要按钮：冷调玫瑰描边 ghost 风格。
/// 用于与主操作按钮（paper-accent 实心）成对的次操作，如「管理文章」「重建内容」。
/// 亮色文字 #83495b vs 米白底 6.5:1、暗色 #cca4b0 vs entry 底 7.46:1，均过 WCAG AA。
pub const BTN_SECONDARY: &str =
    "px-6 py-3 rounded-full text-sm font-medium text-center text-paper-accent-2 border border-paper-accent-2/40 hover:border-paper-accent-2 hover:bg-paper-accent-2-soft transition-all cursor-pointer";

// ===========================================================================
// 组件
// ===========================================================================

/// 分页导航组件。
///
/// 统一了后台与前台的分页 UI，通过 `variant` 切换配色与展示细节：
/// - `"admin"`：灰黑胶囊按钮，显示页码计数（`{当前} / {总} 页 (共 {total} {unit})`），
///   首尾页渲染禁用态。
/// - `"frontend"`：主题绿胶囊按钮，不显示计数，首尾页直接不渲染按钮。
///
/// Props：
/// - `variant`：`"admin"` 或 `"frontend"`
/// - `current_page`：当前页码（从 1 开始）
/// - `total`：数据总条数
/// - `per_page`：每页条数，用于计算总页数
/// - `prev_route`：点击上一页跳转的目标路由
/// - `next_route`：点击下一页跳转的目标路由
/// - `unit`：计数单位（"篇" / "条"），仅 admin 显示计数时使用
#[component]
pub fn Pagination(
    variant: &'static str,
    current_page: i32,
    total: i64,
    per_page: i32,
    prev_route: Route,
    next_route: Route,
    unit: &'static str,
) -> Element {
    let has_prev = current_page > 1;
    let total_pages = ((total + per_page as i64 - 1) / per_page as i64).max(1) as i32;
    let has_next = current_page < total_pages;

    // admin 与 frontend 的配色差异。
    let is_admin = variant == "admin";
    let (link_class, link_extra_next) = if is_admin {
        (
            "inline-flex items-center px-4 py-2 text-sm text-paper-theme bg-paper-accent rounded-full hover:brightness-110 active:scale-[0.98] transition-all duration-200 cursor-pointer",
            "",
        )
    } else {
        (
            "inline-flex items-center px-4 py-2 text-sm text-white bg-paper-accent rounded-full hover:brightness-110 active:scale-[0.98] transition-all duration-200 cursor-pointer",
            "ml-auto",
        )
    };
    let disabled_class =
        "inline-flex items-center px-4 py-2 text-sm text-paper-secondary bg-paper-tertiary rounded-full cursor-not-allowed";

    // admin 首尾页渲染禁用态；frontend 首尾页直接不渲染。
    rsx! {
        nav { class: if is_admin { "flex mt-6 justify-between" } else { "flex mt-10 mb-6 justify-between" },
            if has_prev {
                Link { class: "{link_class}", to: prev_route,
                    span { class: "mr-1", "«" }
                    "上一页"
                }
            } else if is_admin {
                span { class: "{disabled_class}",
                    span { class: "mr-1", "«" }
                    "上一页"
                }
            }

            // admin 显示页码计数。
            if is_admin {
                span { class: "text-sm text-paper-secondary self-center",
                    "{current_page} / {total_pages} 页 (共 {total} {unit})"
                }
            }

            if has_next {
                Link { class: "{link_class} {link_extra_next}", to: next_route,
                    "下一页"
                    span { class: "ml-1", "»" }
                }
            } else if is_admin {
                span { class: "{disabled_class}",
                    "下一页"
                    span { class: "ml-1", "»" }
                }
            }
        }
    }
}

/// 状态徽章组件。
///
/// 外层固定 `BADGE_BASE`，颜色类由调用方传入。之所以用 `color_class` prop
/// 而非枚举变体，是因为部分场景（如回收站剩余天数）的颜色由动态逻辑决定
/// （>7 天中性 / ≤7 天主题绿 / ≤0 琥珀），硬编码 variant 反而不够灵活。
///
/// Props：
/// - `color_class`：背景与文字颜色类（如 `post.status_badge_class()` 的返回值）
/// - `label`：徽章文本
#[component]
pub fn StatusBadge(color_class: &'static str, label: String) -> Element {
    rsx! {
        span { class: "{BADGE_BASE} {color_class}", "{label}" }
    }
}

/// 空状态 / 错误状态组件。
///
/// 用于列表页无数据或加载失败时的居中占位提示。
///
/// Props：
/// - `message`：提示文本
/// - `variant`：`"default"`（灰色，空状态）或 `"error"`（红色，加载失败）
#[component]
pub fn EmptyState(message: &'static str, variant: &'static str) -> Element {
    let class = match variant {
        "error" => "text-center text-red-500 dark:text-red-400 py-20",
        _ => "text-center py-20 text-paper-secondary",
    };
    rsx! {
        div { class: "{class}", "{message}" }
    }
}

/// Tooltip 定位样式（胶囊：黑底白字，hover 显现）。
const TOOLTIP_STYLE: &str =
    "pointer-events-none absolute left-1/2 -translate-x-1/2 px-3 py-1.5 text-xs font-medium whitespace-nowrap rounded-lg opacity-0 group-hover:opacity-100 transition-opacity duration-200 bg-paper-primary text-paper-theme shadow-lg z-50";

/// Tooltip 包裹组件。
///
/// 将任意触发器（按钮等）包裹后，鼠标 hover 时在上方或下方居中弹出提示。
/// 用 CSS `group` + `group-hover:opacity-100` 实现，无 JS 状态，`pointer-events-none`
/// 保证不拦截点击。
///
/// Props：
/// - `tip`：提示文案
/// - `children`：触发器元素（按钮 / 链接等）
/// - `placement`：弹出方向，`"top"`（默认）或 `"bottom"`
///
/// 注意：父容器若有 `overflow-hidden` 会裁掉 tooltip，此时应选朝外的方向
/// （如表格行在 `overflow-hidden` 容器内，朝上的 tooltip 才不会被裁）。
#[component]
pub fn Tooltip(
    tip: String,
    children: Element,
    #[props(default = "top")] placement: &'static str,
) -> Element {
    // 朝上：tooltip 在触发器上方（bottom-full + mb-2）；朝下：在下方（top-full + mt-2）。
    let position_class = if placement == "bottom" {
        "top-full mt-2"
    } else {
        "bottom-full mb-2"
    };
    rsx! {
        div { class: "group relative",
            {children}
            div { class: "{TOOLTIP_STYLE} {position_class}", "{tip}" }
        }
    }
}

static TAB_GROUP_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// 筛选选项卡组件。
///
/// 用于切换不同的视图或筛选条件（例如：全部、待审核、已通过等）。
/// 具备高级的平滑滑动底部指示器动画。
///
/// Props：
/// - `items`：选项卡列表，每一项为 `(value, label)`
/// - `active_value`：当前选中的值
/// - `on_change`：选项卡切换时的回调
#[component]
pub fn FilterTabs(
    items: Vec<(&'static str, &'static str)>,
    active_value: String,
    on_change: EventHandler<String>,
) -> Element {
    #[allow(unused_mut)]
    let mut indicator_style = use_signal(|| "left: 0px; width: 0px; opacity: 0;".to_string());
    let id_prefix = use_hook(|| TAB_GROUP_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst));

    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    let update_indicator = move |active: String| {
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;

                // 等待 DOM 节点更新
                let promise = js_sys::Promise::new(&mut |resolve, _| {
                    if let Some(window) = web_sys::window() {
                        let _ = window
                            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 50);
                    }
                });
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;

                if let Some(window) = web_sys::window() {
                    if let Some(doc) = window.document() {
                        let element_id = format!("tab-{}-{}", id_prefix, active);
                        if let Some(el) = doc.get_element_by_id(&element_id) {
                            if let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>() {
                                let left = html_el.offset_left();
                                let width = html_el.offset_width();
                                indicator_style.set(format!(
                                    "left: {}px; width: {}px; opacity: 1;",
                                    left, width
                                ));
                            }
                        }
                    }
                }
            }
        });
    };

    use_effect({
        let active_value = active_value.clone();
        move || {
            update_indicator(active_value.clone());
        }
    });

    rsx! {
        div { class: "relative flex gap-1 border-b border-paper-border",
            for (value, label) in items {
                button {
                    id: "tab-{id_prefix}-{value}",
                    key: "{value}",
                    class: if active_value == *value { "cursor-pointer px-4 py-2 text-sm font-medium text-paper-primary transition-colors" } else { "cursor-pointer px-4 py-2 text-sm font-medium text-paper-secondary hover:text-paper-primary transition-colors" },
                    onclick: {
                        let v = value.to_string();
                        move |_| {
                            on_change.call(v.clone());
                            update_indicator(v.clone());
                        }
                    },
                    "{label}"
                }
            }
            // 绝对定位的滑动颜色条
            div {
                class: "absolute bottom-[-1px] h-[2px] bg-paper-accent transition-all duration-300 ease-[cubic-bezier(0.4,0,0.2,1)] pointer-events-none",
                style: "{indicator_style}",
            }
        }
    }
}

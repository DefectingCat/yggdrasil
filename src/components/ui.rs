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

/// Admin 卡片容器：内容档圆角（16px），作为主面板内的内容卡片，与外壳 32px 形成层次。
pub const ADMIN_CARD_CLASS: &str = "bg-[var(--color-paper-entry)] rounded-2xl shadow-sm border border-transparent hover:border-[var(--color-paper-border)] transition-colors";

/// Admin 表格容器：内容档圆角（16px），与卡片一致。
pub const ADMIN_TABLE_CLASS: &str = "bg-[var(--color-paper-entry)] rounded-2xl shadow-sm border border-transparent hover:border-[var(--color-paper-border)] transition-colors overflow-hidden";

/// Admin 表格行 hover 态：底部分割线 + 悬停背景。
pub const ADMIN_ROW_HOVER: &str =
    "border-b border-paper-border last:border-b-0 hover:bg-[var(--color-paper-accent-soft)] transition-colors";

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
    "px-4 py-1.5 text-sm font-medium bg-green-500/10 text-green-600 dark:text-green-400 rounded-full hover:bg-green-500/20 transition-colors cursor-pointer";
/// 琥珀色实心小按钮（批量标为垃圾）。
pub const BTN_SOLID_AMBER: &str =
    "px-4 py-1.5 text-sm font-medium bg-amber-500/10 text-amber-600 dark:text-amber-400 rounded-full hover:bg-amber-500/20 transition-colors cursor-pointer";
/// 红色实心小按钮（批量删除、批量彻底删除）。
pub const BTN_SOLID_RED: &str =
    "px-4 py-1.5 text-sm font-medium bg-red-500/10 text-red-600 dark:text-red-400 rounded-full hover:bg-red-500/20 transition-colors cursor-pointer";

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

// --- 次要按钮（Teal 第二色，ghost 描边风格，从属于主色 Green） ---

/// 次要按钮：极简风次要操作。
pub const BTN_SECONDARY: &str =
    "px-6 py-2.5 rounded-full text-sm font-medium text-center text-[var(--color-paper-secondary)] bg-[var(--color-paper-entry)] hover:bg-[var(--color-paper-border)] hover:text-[var(--color-paper-primary)] transition-all cursor-pointer";

// --- 主操作按钮（主题绿实心胶囊，全站统一 CTA） ---

/// 主操作按钮：主题绿实心胶囊（用于 `<Link>`、无 loading 态的静态按钮）。
pub const BTN_PRIMARY: &str =
    "inline-flex items-center justify-center px-5 py-2 text-sm font-medium text-[var(--color-paper-theme)] bg-[var(--color-paper-accent)] rounded-full shadow-sm hover:brightness-110 active:scale-[0.98] transition-all cursor-pointer";

/// 小号主操作按钮：工具栏场景（刷新 / 导出 / 创建备份）。
pub const BTN_PRIMARY_SM: &str =
    "inline-flex items-center justify-center px-4 py-1.5 text-sm font-medium text-[var(--color-paper-theme)] bg-[var(--color-paper-accent)] rounded-full hover:brightness-110 active:scale-[0.98] transition-all cursor-pointer";

// --- 描边按钮 ---

/// 描边次要按钮（posts 重建、system 刷新列表）：`relative` 以承载 spinner 叠加层。
pub const BTN_OUTLINE: &str =
    "relative px-4 py-2 rounded-full text-sm font-medium text-paper-primary border border-paper-border hover:border-paper-accent hover:text-paper-accent transition-all cursor-pointer";

/// 红色描边危险按钮（trash 清空回收站）。
pub const BTN_DANGER_OUTLINE: &str =
    "px-4 py-2 text-sm font-medium text-red-600 dark:text-red-400 border border-red-300 dark:border-red-900/50 rounded-full hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors cursor-pointer";

// --- 图标按钮 ---

/// 关闭图标按钮（× 关闭提示条）。
pub const BTN_CLOSE_ICON: &str =
    "shrink-0 text-red-400 hover:text-red-600 cursor-pointer text-lg leading-none";

/// 方形图标按钮（trash 步进 −/+）。
pub const BTN_ICON: &str =
    "w-9 h-9 flex items-center justify-center text-sm text-paper-secondary hover:text-paper-primary hover:bg-paper-theme cursor-pointer transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-paper-accent/40";

// ===========================================================================
// 组件
// ===========================================================================

/// 分页导航组件。
///
/// 统一了后台与前台的分页 UI，通过 `variant` 切换配色与展示细节：
/// - `"admin"`：描边胶囊按钮（与 `BTN_OUTLINE` 同族），显示页码计数
///   （`{当前} / {总} 页 (共 {total} {unit})`），首尾页渲染禁用态。
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

    // admin 与 frontend 的配色差异。admin 上一页/下一页用描边胶囊，与本页其它
    // 操作按钮（BTN_OUTLINE）同族，避免出现方角实心按钮破坏整体圆角语言。
    let is_admin = variant == "admin";
    let (link_class, link_extra_next) = if is_admin {
        (
            "inline-flex items-center px-4 py-2 text-sm font-medium text-paper-primary border border-paper-border rounded-full hover:border-paper-accent hover:text-paper-accent active:scale-[0.98] transition-all duration-200 cursor-pointer",
            "",
        )
    } else {
        (
            "inline-flex items-center px-4 py-2 text-sm text-white bg-paper-accent rounded-full hover:brightness-110 active:scale-[0.98] transition-all duration-200 cursor-pointer",
            "ml-auto",
        )
    };
    let disabled_class =
        "inline-flex items-center px-4 py-2 text-sm font-medium text-paper-secondary border border-paper-border rounded-full cursor-not-allowed";

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
        div { class: "group relative inline-flex",
            {children}
            div { class: "{TOOLTIP_STYLE} {position_class}", "{tip}" }
        }
    }
}

/// Popover 遮罩与面板的层级(z-40 遮罩 < z-50 面板),与 Tooltip/lightbox 同 z-50。
const POPOVER_OVERLAY_CLASS: &str = "fixed inset-0 z-40";
/// Popover 面板:卡片化大圆角 + 阴影 + 淡入缩放动画。
const POPOVER_PANEL_CLASS: &str =
    "fixed z-50 bg-[var(--color-paper-entry)] rounded-2xl shadow-lg border border-[var(--color-paper-border)] p-4 animate-popover-enter";

/// 受控式通用 Popover(浮层)组件。
///
/// 与 [`Tooltip`] 对称——但 Tooltip 是纯 CSS hover、无状态;Popover 是点击触发、
/// 受控开关,用于承载确认框、轻量表单等交互内容。
///
/// ## 定位策略
///
/// 父容器常有 `overflow-hidden`(如 `ADMIN_TABLE_CLASS`),`position:absolute` 子节点
/// 会被裁掉,故面板用 **`position:fixed`**,以触发点击的**视口坐标**(`MouseEvent::
/// client_coordinates()`)作为锚点(参照 `theme.rs` 圆形展开动画的坐标用法)。无需
/// `getBoundingClientRect`/`node_ref`。
///
/// - `placement: "top"`(默认):面板底边贴点击点上方(`bottom: 100vh - y + gap`)。
/// - `placement: "bottom"`:面板顶边贴点击点下方(`top: y + gap`)。
/// - 水平:面板中心对齐点击点(`left: x` + `-translate-x-1/2`)。
///
/// ## 关闭路径(三条)
///
/// 1. 点遮罩(透明,仅作点击兜底)→ `on_close`。
/// 2. Escape 键 → `on_close`(组件内 `use_effect` 注册全局 keydown 监听,`use_drop` 清理)。
/// 3. 面板内确认/取消按钮调用 `on_close`。
///
/// ## Props
///
/// - `open`:受控开关;`false` 时组件不渲染任何内容(SSR 安全)。
/// - `anchor_x` / `anchor_y`:触发点击的视口坐标。
/// - `placement`:`"top"`(默认)/ `"bottom"`。
/// - `children`:面板内容(确认框等)。
/// - `on_close`:任一关闭路径触发。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
pub fn Popover(
    open: bool,
    anchor_x: i32,
    anchor_y: i32,
    children: Element,
    on_close: EventHandler<()>,
    #[props(default = "top")] placement: &'static str,
) -> Element {
    // Escape 关闭:组件 open 时注册全局 keydown 监听,关闭/卸载时移除。
    // 手写最小 listener 而非复用 use_event_listener——后者 handler 无参,拿不到
    // KeyboardEvent 的 key()。用 use_hook 持有 Closure,use_effect 注册,use_drop 清理。
    #[cfg(target_arch = "wasm32")]
    {
        use dioxus::prelude::{use_drop, use_effect, use_hook};
        use std::cell::RefCell;
        use std::rc::Rc;
        type EscState = Rc<RefCell<Option<wasm_bindgen::prelude::Closure<dyn FnMut(web_sys::KeyboardEvent)>>>>;
        let state: EscState = use_hook(|| Rc::new(RefCell::new(None)));
        let state_for_drop = state.clone();
        let open_for_effect = open;
        let on_close_for_esc = on_close;
        use_effect(move || {
            if !open_for_effect {
                return;
            }
            let Some(window) = web_sys::window() else {
                return;
            };
            let on_close_for_esc = on_close_for_esc;
            // Closure 带 KeyboardEvent 参数:浏览器调用 handler 时传入事件对象,
            // 无需依赖已废弃的 window.event()。as_ref + unchecked_ref 转成 JS Function。
            let closure = wasm_bindgen::prelude::Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "Escape" {
                    on_close_for_esc.call(());
                }
            })
                as Box<dyn FnMut(web_sys::KeyboardEvent)>);
            let _ = window.add_event_listener_with_callback(
                "keydown",
                wasm_bindgen::JsCast::unchecked_ref(closure.as_ref()),
            );
            *state.borrow_mut() = Some(closure);
        });
        use_drop(move || {
            if let Some(closure) = state_for_drop.borrow_mut().take() {
                if let Some(window) = web_sys::window() {
                    let _ = window.remove_event_listener_with_callback(
                        "keydown",
                        wasm_bindgen::JsCast::unchecked_ref(closure.as_ref()),
                    );
                }
            }
        });
    }

    if !open {
        return rsx! {};
    }

    // 面板定位:top/bottom 决定垂直方向;水平统一居中于点击点。
    let style = if placement == "bottom" {
        format!("top: {y}px; left: {x}px; transform: translateX(-50%);", x = anchor_x, y = anchor_y + 8)
    } else {
        // top:面板在点击点上方——用 bottom 锚定 viewport 底,差值即视口高度 - y + 间隙。
        // 视口高度用 100vh,纯 CSS 无需 JS 读取 scrollHeight。
        format!(
            "bottom: calc(100vh - {y}px + 8px); left: {x}px; transform: translateX(-50%);",
            x = anchor_x,
            y = anchor_y,
        )
    };

    rsx! {
        // 透明遮罩:拦截外部点击(点遮罩即关)。z-40 < 面板 z-50。
        div {
            class: "{POPOVER_OVERLAY_CLASS}",
            onclick: move |_| on_close.call(()),
        }
        // 面板:fixed 定位逃出 overflow-hidden 容器。
        div {
            class: "{POPOVER_PANEL_CLASS}",
            style: "{style}",
            {children}
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
                crate::utils::time::sleep_ms(50).await;

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
        div { class: "relative flex gap-4 border-b border-paper-border mb-6",
            for (value, label) in items {
                button {
                    id: "tab-{id_prefix}-{value}",
                    key: "{value}",
                    class: if active_value == *value { "cursor-pointer px-2 py-3 text-xs font-mono tracking-widest uppercase text-paper-primary transition-colors" } else { "cursor-pointer px-2 py-3 text-xs font-mono tracking-widest uppercase text-paper-secondary hover:text-paper-primary transition-colors" },
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
                class: "absolute bottom-[-1px] h-[2px] bg-paper-primary transition-all duration-300 ease-[cubic-bezier(0.4,0,0.2,1)] pointer-events-none",
                style: "{indicator_style}",
            }
        }
    }
}

/// 主操作按钮，内置 loading spinner 叠加（统一三态）。
///
/// 用于全站所有主题绿 CTA（执行 / 刷新 / 创建备份 / 发布 / 保存设置）。
/// 三态：
/// - `loading=true`：主题绿底 + 文字隐藏（`opacity-0`）+ spinner 绝对居中
///   （按钮宽度不变，避免加载时布局抖动）
/// - `disabled=true`（且未 loading）：灰色底（`bg-paper-tertiary`）+
///   `cursor-not-allowed`
/// - 正常：主题绿底 + `hover:brightness-110` + `active:scale-[0.98]`
///
/// Props：
/// - `label`：正常态显示的文案（loading 时隐藏，由 spinner 占位）
/// - `loading`：是否处于加载态
/// - `disabled`：是否禁用（loading 优先级更高）
/// - `variant`：`"primary"`（默认，`px-5 py-2`）或 `"sm"`（`px-4 py-1.5`）
/// - `onclick`：点击回调
#[component]
pub fn LoadingButton(
    label: String,
    loading: bool,
    #[props(default = false)] disabled: bool,
    #[props(default = "primary")] variant: &'static str,
    onclick: EventHandler<()>,
) -> Element {
    // 尺寸变体：sm 用于工具栏（刷新/导出），primary 用于主 CTA（执行/发布/保存）。
    let size = if variant == "sm" {
        "px-4 py-1.5"
    } else {
        "px-5 py-2 shadow-sm"
    };

    // 三态背景：loading 与正常都是主题绿（保持视觉连续），disabled 灰化。
    let (bg, cursor) = if disabled && !loading {
        (
            "bg-[var(--color-paper-tertiary)] text-[var(--color-paper-secondary)]",
            "cursor-not-allowed",
        )
    } else {
        (
            "text-[var(--color-paper-theme)] bg-[var(--color-paper-accent)] hover:brightness-110 active:scale-[0.98]",
            "cursor-pointer",
        )
    };

    rsx! {
        button {
            class: "relative inline-flex items-center justify-center {size} {bg} {cursor} rounded-full text-sm font-medium transition-all",
            disabled: loading || disabled,
            onclick: move |_| onclick.call(()),
            span { class: if loading { "opacity-0" } else { "" }, "{label}" }
            if loading {
                span {
                    class: "absolute inset-0 flex items-center justify-center",
                    dangerous_inner_html: SPINNER_SVG,
                }
            }
        }
    }
}

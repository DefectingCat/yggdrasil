//! 表单控件组件
//!
//! 提供登录、注册、评论等页面共享的输入框、按钮与提示框样式常量与组件。

use dioxus::prelude::*;

/// 输入框基础 CSS 类，统一文本框、邮箱框、URL 框等样式。
pub const INPUT_CLASS: &str = "w-full px-4 py-2 border border-paper-border rounded-2xl bg-paper-entry text-paper-primary placeholder:text-paper-tertiary focus:outline-none focus:border-paper-accent focus:ring-1 focus:ring-paper-accent/30 transition-colors duration-200";

/// 主按钮 CSS 类，用于表单提交等主操作按钮。
pub const BUTTON_PRIMARY_CLASS: &str = "w-full py-2.5 px-4 bg-paper-accent text-white font-medium rounded-full hover:brightness-110 active:scale-[0.98] transition-all duration-200 cursor-pointer";

/// FormSelect 实例 id 计数器（跨泛型单例化全局唯一）。
static FORM_SELECT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// FormSelect 紧凑触发器样式：工具栏内联小下拉（自动刷新、导出格式等）。
/// 自动宽度 + text-sm + 小圆角，chevron 与面板样式与默认表单款一致。
pub const FORM_SELECT_COMPACT_CLASS: &str = "inline-flex w-auto cursor-pointer select-none text-left text-sm pl-3 pr-8 py-1 border border-paper-border rounded-lg bg-paper-theme text-paper-primary focus:outline-none focus:border-paper-accent focus:ring-1 focus:ring-paper-accent/30 transition-colors duration-200";

/// 面板应向上展开的条件：视口下方空间不足，且上方空间比下方更宽余。
///
/// 纯函数便于单测；`panel_height` 由调用方按选项数估算（见 `measure_flip`）。
#[allow(dead_code)] // server 构建下仅被 dead 的组件体引用（wasm 与单测为真实调用方）
fn should_flip(trigger_top: f64, trigger_bottom: f64, viewport_height: f64, panel_height: f64) -> bool {
    /// 面板与触发器的间隙（mt-1.5）加视口边缘留白。
    const MARGIN: f64 = 14.0;
    let below = viewport_height - trigger_bottom;
    let above = trigger_top;
    below < panel_height + MARGIN && above > below
}

/// 键盘导航的循环索引：在 `len` 个选项中从 `cur` 移动 `delta`（±1），越界回绕。
fn wrap_index(cur: usize, delta: i32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (cur as i32 + delta).rem_euclid(len as i32) as usize
}

/// 测量触发器视口位置，决定面板展开方向（仅 wasm；SSR 无 DOM）。
#[cfg(target_arch = "wasm32")]
fn measure_flip(trigger_id: &str, option_count: usize) -> bool {
    /// 选项行高：24px 行盒 + py-2.5（20px 垂直内边距）。
    const ROW_HEIGHT: f64 = 44.0;
    /// 面板 1px 边框 ×2 + p-1.5 内边距。
    const PANEL_CHROME: f64 = 14.0;
    /// 面板高度上限：max-h-60（240px）+ PANEL_CHROME。
    const PANEL_MAX: f64 = 254.0;

    let Some(window) = web_sys::window() else {
        return false;
    };
    let Some(document) = window.document() else {
        return false;
    };
    let Some(el) = document.get_element_by_id(trigger_id) else {
        return false;
    };
    let rect = el.get_bounding_client_rect();
    let viewport = window
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0);
    let panel_height = ((option_count as f64) * ROW_HEIGHT + PANEL_CHROME).min(PANEL_MAX);
    should_flip(rect.top(), rect.bottom(), viewport, panel_height)
}

/// 把指定选项滚入面板可视区（打开/键盘导航时跟随；仅 wasm）。
#[cfg(target_arch = "wasm32")]
fn scroll_option_into_view(element_id: &str) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Some(el) = document.get_element_by_id(element_id) else {
        return;
    };
    let opts = web_sys::ScrollIntoViewOptions::new();
    opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
    el.scroll_into_view_with_scroll_into_view_options(&opts);
}

/// 下拉选择框组件（自定义弹层，全主题化）。
///
/// 原生 `<select>` 的弹出列表由 OS/浏览器渲染，无法跟随主题（暗色下是白底
/// 系统菜单），故用 `button[aria-haspopup=listbox]` + 绝对定位面板重写：
/// - 面板/选项全部使用 Catppuccin 语义色，带 `select-enter` 入场动画；
/// - focus 始终留在触发器，键盘经 `aria-activedescendant` 高亮（↑↓ 循环、
///   Enter/Space 选中、Esc 关闭、Home/End 跳首尾、Tab 关闭并自然流转焦点）；
/// - 透明遮罩拦截外部点击关闭（同 Popover 模式）；选项 `onmousedown` 阻止默认
///   行为，点击不夺走触发器焦点；
/// - 打开时视口下方空间不足且上方更宽余则向上展开（`should_flip`）；
/// - 泛型值绑定：`onchange` 直接回传选中项的 `T`，无需字符串反查。
///
/// Props：
/// - `id`：触发器 id，用于与 label 关联（缺省用内部计数器生成）
/// - `value`：当前选中项（受控）
/// - `options`：可选项 `(值, 标签)` 列表
/// - `onchange`：选中变化回调，回传新选中项的值
#[component]
pub fn FormSelect<T: Clone + PartialEq + 'static>(
    id: Option<String>,
    value: T,
    options: Vec<(T, &'static str)>,
    onchange: EventHandler<T>,
    /// 触发器样式覆盖：缺省为全宽表单款；工具栏内联场景传
    /// [`FORM_SELECT_COMPACT_CLASS`]，或自定义类串（如编辑器底部胶囊）。
    #[props(default)]
    trigger_class: Option<&'static str>,
) -> Element {
    // 面板与 POPOVER_PANEL_CLASS 同源（卡片化圆角 + 阴影）。宽度取 max(触发器,
    // 最长选项)：紧凑触发器（如“手动”）下面板仍能完整展示长选项；上限防出屏。
    // 水平以触发器中心居中：面板宽于触发器时两侧对称探出，窄触发器不失衡。
    // 居中用 [transform:translateX(-50%)] 而非 -translate-x-1/2 utility：Tailwind
    // v4 的 translate utility 走独立 translate 属性，会与 select-enter 关键帧的
    // transform 叠加造成双倍位移；关键帧的 fill 值与此处 transform 完全一致。
    // 定义在函数体内：模块级私有常量若仅被 wasm 门控调用点引用，会在 server
    // 构建下触发 dead_code。
    const TRIGGER_CLASS: &str = "w-full block cursor-pointer truncate select-none text-left pl-4 pr-10 py-2 border border-paper-border rounded-2xl bg-paper-entry text-paper-primary focus:outline-none focus:border-paper-accent focus:ring-1 focus:ring-paper-accent/30 transition-colors duration-200";
    const PANEL_CLASS: &str = "absolute left-1/2 z-50 w-max min-w-full max-w-[calc(100vw_-_2rem)] [transform:translateX(-50%)] max-h-60 overflow-y-auto rounded-2xl border border-[var(--color-paper-border)] bg-[var(--color-paper-entry)] p-1.5 shadow-lg animate-select-enter";

    let trigger_cls = trigger_class.unwrap_or(TRIGGER_CLASS);

    let id_prefix = use_hook(|| FORM_SELECT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst));

    // 受控选中序号；value 不在 options 时兜底 0（与浏览器默认选中第一项一致）。
    let selected = options.iter().position(|(v, _)| *v == value).unwrap_or(0);
    let selected_label = options.get(selected).map(|(_, l)| *l).unwrap_or_default();
    let len = options.len();

    let mut open = use_signal(|| false);
    let mut active = use_signal(|| selected);
    // flip_up 的 set 均在 wasm 门控语句内（宿主构建下只读），参照 FilterTabs 先例。
    #[allow(unused_mut)]
    let mut flip_up = use_signal(|| false);

    // onkeydown 闭包与下方选项渲染各需一份 options（键盘选中回查 / 渲染）。
    let options_for_keys = options.clone();

    // 触发器 id：外部未指定时用内部前缀生成，ARIA 关联统一走它。
    let trigger_id = id.unwrap_or_else(|| format!("form-select-{id_prefix}"));
    // 两个事件闭包各持一份（仅 wasm 用于 flip 测量按 id 查元素）。
    #[cfg(target_arch = "wasm32")]
    let trigger_id_click = trigger_id.clone();
    #[cfg(target_arch = "wasm32")]
    let trigger_id_keys = trigger_id.clone();

    // 打开或键盘导航时，把高亮项滚入面板可视区。
    use_effect(move || {
        if open() {
            #[cfg(target_arch = "wasm32")]
            {
                let idx = active();
                scroll_option_into_view(&format!("form-select-{id_prefix}-opt-{idx}"));
            }
        }
    });

    // 预计算每行展示态，rsx 循环内只做移动与闭包捕获。
    let active_idx = active();
    let rows: Vec<(usize, T, &'static str, &'static str, &'static str)> = options
        .iter()
        .enumerate()
        .map(|(i, (v, l))| {
            let highlight = if i == active_idx {
                "bg-[var(--color-paper-accent-soft)]"
            } else {
                ""
            };
            let text = if i == selected {
                "text-paper-accent"
            } else {
                "text-[var(--color-paper-primary)]"
            };
            (i, v.clone(), *l, highlight, text)
        })
        .collect();

    let chevron_rotate = if open() { "rotate-180" } else { "" };
    let placement_cls = if flip_up() {
        "bottom-full mb-1.5 origin-bottom"
    } else {
        "top-full mt-1.5 origin-top"
    };
    let active_descendant = open().then(|| {
        let idx = active();
        format!("form-select-{id_prefix}-opt-{idx}")
    });

    rsx! {
        div { class: "relative",
            button {
                id: "{trigger_id}",
                r#type: "button",
                class: "{trigger_cls}",
                aria_haspopup: "listbox",
                aria_expanded: "{open()}",
                aria_activedescendant: active_descendant,
                onclick: move |_| {
                    // 打开态下触发器被透明遮罩盖住，点击落在遮罩上即关闭；
                    // 这里只需处理「未开 → 开」。
                    if !open() {
                        #[cfg(target_arch = "wasm32")]
                        flip_up.set(measure_flip(&trigger_id_click, len));
                        active.set(selected);
                        open.set(true);
                    }
                },
                onkeydown: move |e| {
                    let key = e.key();
                    let is_space = matches!(&key, Key::Character(s) if s == " ");
                    if !open() {
                        if key == Key::ArrowDown || key == Key::ArrowUp || key == Key::Enter || is_space {
                            e.prevent_default();
                            #[cfg(target_arch = "wasm32")]
                            flip_up.set(measure_flip(&trigger_id_keys, len));
                            active.set(selected);
                            open.set(true);
                        }
                        return;
                    }
                    if key == Key::ArrowDown {
                        e.prevent_default();
                        active.set(wrap_index(active(), 1, len));
                    } else if key == Key::ArrowUp {
                        e.prevent_default();
                        active.set(wrap_index(active(), -1, len));
                    } else if key == Key::Home {
                        e.prevent_default();
                        active.set(0);
                    } else if key == Key::End {
                        e.prevent_default();
                        active.set(len.saturating_sub(1));
                    } else if key == Key::Enter || is_space {
                        e.prevent_default();
                        if let Some((v, _)) = options_for_keys.get(active()) {
                            onchange.call(v.clone());
                        }
                        open.set(false);
                    } else if key == Key::Escape {
                        e.prevent_default();
                        open.set(false);
                    } else if key == Key::Tab {
                        // 不拦截：关闭后焦点自然流转到下一个控件。
                        open.set(false);
                    }
                },
                "{selected_label}"
                // 下拉箭头（打开时翻转）
                svg {
                    class: "pointer-events-none absolute right-4 top-1/2 -translate-y-1/2 w-4 h-4 text-paper-secondary transition-transform duration-200 {chevron_rotate}",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    path {
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        d: "M6 9l6 6 6-6",
                    }
                }
            }

            if open() {
                // 透明遮罩：拦截外部点击（点遮罩即关），z-40 < 面板 z-50。
                div {
                    class: "fixed inset-0 z-40",
                    onclick: move |_| open.set(false),
                }
                ul {
                    class: "{PANEL_CLASS} {placement_cls}",
                    role: "listbox",
                    aria_labelledby: "{trigger_id}",
                    for (i, opt_value, opt_label, highlight_cls, text_cls) in rows {
                        li {
                            id: "form-select-{id_prefix}-opt-{i}",
                            class: "flex items-center justify-between gap-2 px-3 py-2.5 rounded-xl cursor-pointer select-none transition-colors hover:bg-[var(--color-paper-accent-soft)] {text_cls} {highlight_cls}",
                            role: "option",
                            aria_selected: "{i == selected}",
                            // 阻止 mousedown 默认行为：点击选项不夺走触发器焦点。
                            onmousedown: move |e| e.prevent_default(),
                            onclick: move |_| {
                                onchange.call(opt_value.clone());
                                open.set(false);
                            },
                            onmouseenter: move |_| active.set(i),
                            span { class: "truncate", "{opt_label}" }
                            if i == selected {
                                svg {
                                    class: "w-4 h-4 flex-shrink-0",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        d: "M20 6L9 17l-5-5",
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 表单输入框组件。
///
/// Props：
/// - `id`：input 元素 id，用于与 label 关联
/// - `r#type`：input 类型（如 `"text"`、`"email"`、`"password"`）
/// - `placeholder`：占位提示文本
/// - `value`：当前值
/// - `disabled`：是否禁用（可选，缺省 `false`）
/// - `oninput`：输入事件回调，返回新的字符串值
/// - `onkeydown`：可选的键盘事件回调
#[component]
pub fn FormInput(
    id: Option<String>,
    r#type: &'static str,
    placeholder: &'static str,
    value: String,
    #[props(default)]
    disabled: bool,
    oninput: EventHandler<String>,
    #[props(default)]
    onkeydown: Option<EventHandler<KeyboardEvent>>,
) -> Element {
    let disabled_class = if disabled {
        "opacity-60 cursor-not-allowed"
    } else {
        ""
    };
    rsx! {
        input {
            id: id.unwrap_or_default(),
            class: "{INPUT_CLASS} {disabled_class}",
            r#type: "{r#type}",
            placeholder: "{placeholder}",
            value: "{value}",
            disabled,
            oninput: move |e| oninput.call(e.value()),
            onkeydown: move |e| {
                if let Some(ref handler) = onkeydown {
                    handler.call(e);
                }
            },
        }
    }
}

/// 表单标签组件。
///
/// Props：
/// - `label`：标签文本
/// - `html_for`：关联的 input id
#[component]
pub fn FormLabel(label: &'static str, html_for: Option<String>) -> Element {
    rsx! {
        label {
            class: "block text-sm font-medium text-paper-secondary mb-1",
            r#for: html_for.unwrap_or_default(),
            "{label}"
        }
    }
}

/// 提示框组件，用于显示成功、错误等状态消息。
///
/// Props：
/// - `message`：提示文本
/// - `variant`：风格类型，支持 `"error"`、`"success"` 与其他默认类型
#[component]
pub fn AlertBox(message: String, variant: &'static str) -> Element {
    let (bg_class, text_class) = match variant {
        "error" => (
            "bg-red-100 dark:bg-red-900/30",
            "text-red-700 dark:text-red-300",
        ),
        "success" => (
            "bg-green-100 dark:bg-green-900/30",
            "text-green-700 dark:text-green-300",
        ),
        _ => ("bg-paper-code-bg", "text-paper-secondary"),
    };
    rsx! {
        div { class: "mb-4 p-3 {bg_class} {text_class} rounded-lg text-center", "{message}" }
    }
}

#[cfg(test)]
mod tests {
    use super::{should_flip, wrap_index};

    #[test]
    fn wrap_index_cycles_both_directions() {
        assert_eq!(wrap_index(0, 1, 3), 1);
        assert_eq!(wrap_index(2, 1, 3), 0); // 末尾前进回绕到首
        assert_eq!(wrap_index(0, -1, 3), 2); // 首位后退回绕到尾
        assert_eq!(wrap_index(1, -1, 3), 0);
    }

    #[test]
    fn wrap_index_empty_is_zero() {
        assert_eq!(wrap_index(5, 1, 0), 0); // 空列表不越界
    }

    #[test]
    fn should_flip_only_when_below_insufficient_and_above_wider() {
        // 下方充足：不翻（below = 800-140 = 660 > 200+14）
        assert!(!should_flip(100.0, 140.0, 800.0, 200.0));
        // 下方不足且上方更宽：上翻（below = 160 < 214，above = 600 > 160）
        assert!(should_flip(600.0, 640.0, 800.0, 200.0));
        // 下方不足但上方更窄：保持向下（above = 30 < below = 190）
        assert!(!should_flip(30.0, 70.0, 260.0, 200.0));
        // 恰好放得下（below = 214 == 200+14，非严格小于）：不翻
        assert!(!should_flip(300.0, 340.0, 554.0, 200.0));
    }
}

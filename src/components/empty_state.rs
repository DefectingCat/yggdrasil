//! 空状态组件。
//!
//! 当列表（首页、标签、搜索等）无数据时展示：线条插画配图 + 标题 + 副文案，
//! 可选行动按钮。视觉语言沿用项目 Forest 调色板（鼠尾草绿强调色）与
//! Source Serif 4 衬线标题，留白克制，与首页 HomeInfo 标题区风格一致。
//!
//! 配图为「线条小狗」主题的内联单色 SVG：双手持相机的构图，取景器内两只小狗。
//! 描边使用 `currentColor` 自动适配明暗主题；取景器屏幕区填 `accent-soft` 作视觉焦点，
//! 与标签 hover、分页等处强调色用法保持一致。

use dioxus::prelude::*;
use dioxus::router::components::Link;

/// 空状态行动按钮。
#[derive(Props, Clone, PartialEq)]
pub struct EmptyStateAction {
    /// 按钮文案。
    pub label: &'static str,
    /// 跳转目标路由。
    pub to: crate::router::Route,
}

/// 空状态组件。
///
/// 默认渲染「线条小狗」配图；提供 `title` / `description` / `action` 可覆盖默认文案。
/// 所有元素垂直居中，配图下方留白，与首页 `HomeInfo` 的居中布局对齐。
///
/// Props 由 `#[component]` 宏自动生成（`title` / `description` / `action` 均为可选）。
#[component]
pub fn EmptyState(
    /// 主标题（通常为「还没有文章」之类）。
    #[props(default)] title: Option<&'static str>,
    /// 副文案，说明当前状态或引导用户。
    #[props(default)] description: Option<&'static str>,
    /// 可选的行动按钮。
    #[props(default)] action: Option<EmptyStateAction>,
) -> Element {
    rsx! {
        div { class: "flex flex-col items-center justify-center text-center py-20 px-4 page-enter",
            // 配图：线条小狗。viewBox 固定，宽度用 max-w 限制，高度自适应。
            LineDog {}
            // 主标题：衬线字体，与首页 H1 风格呼应但更轻量。
            h2 { class: "mt-8 text-2xl font-bold tracking-tight text-paper-primary",
                {title.unwrap_or("还没有文章")}
            }
            // 副文案：次要色，限宽保证可读性。
            if let Some(desc) = description {
                p { class: "mt-3 text-sm leading-relaxed text-paper-secondary max-w-md",
                    {desc}
                }
            }
            // 行动按钮：药丸形，与搜索页主按钮一致。
            if let Some(act) = action {
                Link {
                    class: "mt-8 inline-flex items-center px-6 py-2 bg-paper-accent text-white rounded-full font-medium text-sm hover:brightness-110 active:scale-[0.98] transition-all duration-200",
                    to: act.to,
                    {act.label}
                }
            }
        }
    }
}

/// 线条小狗配图（双手持相机，取景器内两只小狗）。
///
/// 纯描边插画，`currentColor` 继承文字色；取景器屏幕填强调色作焦点。
/// `text-paper-tertiary` 提供比正文更浅的线条色，避免抢夺标题视觉权重。
#[component]
fn LineDog() -> Element {
    rsx! {
        svg {
            class: "w-48 h-auto text-paper-tertiary",
            view_box: "0 0 240 200",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            "aria-hidden": true,
            // ── 相机机身 ──
            // 主体圆角矩形。
            rect {
                x: "50", y: "70", width: "140", height: "95", rx: "12",
                stroke: "currentColor", stroke_width: "3.5", stroke_linejoin: "round",
            }
            // 顶部凸起（取景器/闪光灯舱）。
            rect {
                x: "95", y: "58", width: "50", height: "14", rx: "5",
                stroke: "currentColor", stroke_width: "3.5", stroke_linejoin: "round",
            }
            // 快门按钮（机身顶右）。
            circle { cx: "172", cy: "66", r: "6", stroke: "currentColor", stroke_width: "3.5" }

            // ── 取景器屏幕区（视觉焦点，填强调色软背景） ──
            rect {
                x: "66", y: "86", width: "108", height: "63", rx: "6",
                fill: "var(--color-paper-accent-soft)",
                stroke: "currentColor", stroke_width: "2.5",
            }

            // ── 屏幕内：两只小狗 ──
            g {
                stroke: "var(--color-paper-accent)",
                stroke_width: "2.5",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                fill: "none",

                // 左小狗（坐姿，戴项圈）。
                // 头部。
                circle { cx: "97", cy: "110", r: "9" }
                // 左耳（下垂）。
                path { d: "M89 108 q-4 4 -3 9 q4 1 5 -3" }
                // 右耳。
                path { d: "M105 108 q4 4 3 9 q-4 1 -5 -3" }
                // 眼睛。
                circle { cx: "94", cy: "109", r: "0.9", fill: "var(--color-paper-accent)", stroke: "none" }
                circle { cx: "100", cy: "109", r: "0.9", fill: "var(--color-paper-accent)", stroke: "none" }
                // 嘴。
                path { d: "M95 114 q2 2 4 0" }
                // 身体。
                path { d: "M90 119 q-4 8 -2 16 l18 0 q2 -8 -2 -16" }
                // 项圈。
                path { d: "M88 122 h18" }
                // 项圈铭牌。
                circle { cx: "97", cy: "125", r: "1.4", fill: "var(--color-paper-accent)", stroke: "none" }
                // 前腿。
                path { d: "M92 135 l0 6" }
                path { d: "M102 135 l0 6" }

                // 右小狗（坐姿，稍小）。
                circle { cx: "138", cy: "113", r: "8" }
                path { d: "M131 111 q-3 4 -2 8 q3 1 4 -3" }
                path { d: "M145 111 q3 4 2 8 q-3 1 -4 -3" }
                circle { cx: "135.5", cy: "112", r: "0.8", fill: "var(--color-paper-accent)", stroke: "none" }
                circle { cx: "140.5", cy: "112", r: "0.8", fill: "var(--color-paper-accent)", stroke: "none" }
                path { d: "M136 117 q2 1.5 4 0" }
                path { d: "M132 121 q-3 7 -1 14 l14 0 q2 -7 -1 -14" }
                path { d: "M133 119 h12" }
                path { d: "M134 133 l0 5" }
                path { d: "M142 133 l0 5" }
            }

            // ── 镜头（机身正面，屏幕下方代表镜头盖区域） ──
            circle { cx: "120", cy: "150", r: "7", stroke: "currentColor", stroke_width: "3" }
            circle { cx: "120", cy: "150", r: "2.5", fill: "currentColor" }

            // ── 双手（从底部托住相机） ──
            g {
                stroke: "currentColor",
                stroke_width: "3.5",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                fill: "none",
                // 左手。
                path { d: "M50 165 q-6 2 -8 12 q0 6 6 8 q8 1 14 -3" }
                // 左手手指。
                path { d: "M56 172 q2 -4 6 -4" }
                path { d: "M60 178 q3 -3 7 -2" }
                // 右手。
                path { d: "M190 165 q6 2 8 12 q0 6 -6 8 q-8 1 -14 -3" }
                // 右手手指。
                path { d: "M184 172 q-2 -4 -6 -4" }
                path { d: "M180 178 q-3 -3 -7 -2" }
            }
        }
    }
}

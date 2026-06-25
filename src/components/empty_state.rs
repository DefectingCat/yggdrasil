//! 空状态组件。
//!
//! 当列表（首页、标签、搜索等）无数据时展示：插画配图 + 标题 + 副文案，
//! 可选行动按钮。视觉语言沿用项目 Forest 调色板（鼠尾草绿强调色）与
//! Source Serif 4 衬线标题，留白克制，与首页 HomeInfo 标题区风格一致。
//!
//! 配图为「线条小狗」插画（`public/images/xiaotiaoxiaogou_01.webp`），
//! 通过 `<img>` 引用绝对路径，由 Dioxus 的静态资源服务直接返回。

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
            // 配图：线条小狗（双手持相机，取景器内两只小狗）。
            img {
                class: "w-48 h-auto select-none",
                src: "/images/xiaotiaoxiaogou_01.webp",
                alt: "线条小狗插画",
                draggable: "false",
            }
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

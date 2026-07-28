//! 文章卡片骨架屏
//!
//! 模拟 PostCard 组件的视觉占位，用于列表页加载。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;

/// 文章卡片骨架屏组件。
///
/// 包含：标题行(24px bold) + 摘要两行 + 元信息行(日期+标签)。
#[component]
pub fn PostCardSkeleton() -> Element {
    rsx! {
        article { class: "mb-12 flex flex-col bg-[var(--color-paper-entry)] rounded-[2rem] border border-transparent overflow-hidden",
            div { class: "p-8 flex flex-col gap-3",
                // 标题占位 (模拟 h2 text-2xl/3xl font-extrabold)
                SkeletonBox { class: "h-7 w-3/4 rounded" }
                // 摘要两行 (模拟 text-base line-clamp-2)
                SkeletonBox { class: "h-4 w-full rounded" }
                SkeletonBox { class: "h-4 w-5/6 rounded" }
                // 元信息行 (日期 + 分隔 + 标签，模拟 text-sm)
                div { class: "flex flex-wrap items-center gap-3 mt-4",
                    SkeletonBox { class: "h-3.5 w-20 rounded" }
                    SkeletonBox { class: "h-3.5 w-1 rounded" }
                    SkeletonBox { class: "h-3.5 w-16 rounded" }
                }
            }
        }
    }
}

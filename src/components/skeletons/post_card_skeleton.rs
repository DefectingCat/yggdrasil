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
        article { class: "mb-6 p-6 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700",
            // 标题占位 (模拟 h2 text-2xl font-bold)
            SkeletonBox { class: "h-7 w-3/4 rounded mb-3" }
            // 摘要第一行
            SkeletonBox { class: "h-4 w-full rounded mb-2" }
            // 摘要第二行
            SkeletonBox { class: "h-4 w-5/6 rounded mb-3" }
            // 元信息行 (日期 + 标签)
            div { class: "flex items-center gap-3 mt-3",
                SkeletonBox { class: "h-3.5 w-20 rounded" }
                SkeletonBox { class: "h-3.5 w-1 rounded" }
                SkeletonBox { class: "h-3.5 w-16 rounded" }
            }
        }
    }
}

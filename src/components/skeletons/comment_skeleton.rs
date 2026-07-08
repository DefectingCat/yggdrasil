//! 评论列表骨架屏
//!
//! 在评论数据加载期间展示评论条目占位，结构和间距与真实评论列表一致。

use crate::components::skeletons::atoms::*;
use dioxus::prelude::*;

/// 评论列表骨架屏组件。
///
/// 渲染若干条评论条目占位，包含头像、作者名与内容行，样式与真实评论项对齐。
#[component]
pub fn CommentListSkeleton() -> Element {
    rsx! {
        div { class: "divide-y divide-gray-100 dark:divide-gray-700",
            // 第一条评论占位（顶层评论）
            div { class: "py-4",
                div { class: "flex gap-3",
                    SkeletonBox { class: "w-8 h-8 rounded-full shrink-0 mt-0.5" }
                    div { class: "flex-1 space-y-2",
                        SkeletonBox { class: "h-4 w-24 rounded" }
                        SkeletonBox { class: "h-3.5 w-full rounded" }
                        SkeletonBox { class: "h-3.5 w-2/3 rounded" }
                    }
                }
            }
            // 第二条评论占位（子回复，缩进 ml-6 即 24px）
            div { class: "py-4 ml-6",
                div { class: "flex gap-3",
                    SkeletonBox { class: "w-8 h-8 rounded-full shrink-0 mt-0.5" }
                    div { class: "flex-1 space-y-2",
                        SkeletonBox { class: "h-4 w-20 rounded" }
                        SkeletonBox { class: "h-3.5 w-3/4 rounded" }
                    }
                }
            }
            // 第三条评论占位（另一条顶层评论）
            div { class: "py-4",
                div { class: "flex gap-3",
                    SkeletonBox { class: "w-8 h-8 rounded-full shrink-0 mt-0.5" }
                    div { class: "flex-1 space-y-2",
                        SkeletonBox { class: "h-4 w-28 rounded" }
                        SkeletonBox { class: "h-3.5 w-full rounded" }
                        SkeletonBox { class: "h-3.5 w-1/2 rounded" }
                    }
                }
            }
        }
    }
}

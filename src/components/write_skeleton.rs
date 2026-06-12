//! 文章编辑器页骨架屏
//!
//! 在写文章/编辑文章页面加载时展示，模拟标题、元信息、编辑器工具栏与正文区域。

use crate::components::skeletons::atoms::*;
use dioxus::prelude::*;

/// 文章编辑器页骨架屏组件。
///
/// 模拟 Write 页面的整体结构：顶部标题与元信息、中间编辑器区域、底部操作按钮。
#[component]
pub fn WriteSkeleton() -> Element {
    rsx! {
        div { class: "relative flex flex-col flex-1 min-h-0 overflow-hidden",
            div { class: "flex-shrink-0 space-y-5 pt-8",
                SkeletonBox { class: "h-12 w-2/3 rounded-lg" }

                SkeletonBox { class: "h-14 w-full rounded-lg" }

                div { class: "flex flex-wrap items-end gap-x-8 gap-y-4 text-sm",
                    for _ in 0..3 {
                        div { class: "flex-1 min-w-[140px]",
                            SkeletonBox { class: "h-[11px] w-10 rounded mb-2" }
                            SkeletonBox { class: "h-5 w-full rounded" }
                        }
                    }
                }
            }

            div { class: "flex-1 min-h-0 flex flex-col my-4",
                div { class: "flex-1 min-h-0 w-full border border-[var(--color-paper-border)] rounded-xl overflow-hidden bg-white dark:bg-[#2e2e33] shadow-[0_2px_8px_rgba(0,0,0,0.04)] dark:shadow-none space-y-4 p-4",
                    div { class: "flex gap-2 pb-3 border-b border-[var(--color-paper-border)]",
                        for _ in 0..8 {
                            SkeletonBox { class: "w-8 h-8 rounded" }
                        }
                    }
                    div { class: "space-y-3 pt-2",
                        SkeletonBox { class: "h-4 w-[90%] rounded" }
                        SkeletonBox { class: "h-4 w-full rounded" }
                        SkeletonBox { class: "h-4 w-[85%] rounded" }
                        SkeletonBox { class: "h-4 w-[95%] rounded" }
                        SkeletonBox { class: "h-4 w-[60%] rounded" }
                        SkeletonBox { class: "h-4 w-full rounded" }
                        SkeletonBox { class: "h-4 w-[75%] rounded" }
                        SkeletonBox { class: "h-4 w-[80%] rounded" }
                        div { class: "h-4" }
                        SkeletonBox { class: "h-4 w-[70%] rounded" }
                        SkeletonBox { class: "h-4 w-full rounded" }
                        SkeletonBox { class: "h-4 w-[90%] rounded" }
                    }
                }
            }

            div { class: "flex-shrink-0 flex items-center gap-2 pt-2 pb-4",
                SkeletonBox { class: "h-9 w-[56px] rounded-xl" }
                div { class: "w-px h-5 bg-[var(--color-paper-border)]" }
                SkeletonBox { class: "h-9 w-[72px] rounded-xl" }
                div { class: "w-px h-5 bg-[var(--color-paper-border)]" }
                SkeletonBox { class: "h-9 w-[56px] rounded-xl" }
            }
        }
    }
}

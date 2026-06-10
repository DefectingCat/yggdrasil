use dioxus::prelude::*;
use crate::components::skeletons::atoms::*;

#[component]
pub fn WriteSkeleton() -> Element {
    rsx! {
        div { class: "relative flex flex-col flex-1 min-h-0 overflow-hidden",
            // 顶部元信息骨架 - 固定高度
            div { class: "flex-shrink-0 space-y-5 pt-8",
                // 标题骨架
                SkeletonBox { class: "h-12 w-2/3 rounded-lg" }

                // 摘要骨架
                SkeletonBox { class: "h-14 w-full rounded-lg" }

                // 元数据行骨架
                div { class: "flex flex-wrap items-end gap-x-8 gap-y-4",
                    for _ in 0..3 {
                        div { class: "flex-1 min-w-[140px] space-y-2",
                            SkeletonBox { class: "h-3 w-12 rounded" }
                            SkeletonBox { class: "h-8 w-full rounded-lg" }
                        }
                    }
                }

                // 分隔线
                div { class: "h-px bg-[var(--color-paper-tertiary)]" }
            }

            // 编辑器骨架 - 沾满剩余高度
            div { class: "flex-1 min-h-0 flex flex-col my-4",
                div { class: "flex-1 min-h-0 w-full rounded-xl border border-[var(--color-paper-border)] bg-[var(--color-paper-entry)] space-y-4 p-4",
                    // 编辑器工具栏骨架
                    div { class: "flex gap-2 pb-3 border-b border-[var(--color-paper-border)]",
                        for _ in 0..8 {
                            SkeletonBox { class: "w-8 h-8 rounded" }
                        }
                    }
                    // 编辑器内容骨架
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

            // 按钮行骨架
            div { class: "flex-shrink-0 flex items-center gap-2 pt-2 pb-4",
                SkeletonBox { class: "h-9 w-[60px] rounded-full" }
                SkeletonBox { class: "h-9 w-[80px] rounded-full" }
                SkeletonBox { class: "h-9 w-[60px] rounded-full" }
            }
        }
    }
}

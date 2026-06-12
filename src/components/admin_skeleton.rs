use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;

/// 仅仪表盘内容区骨架（不含 header/footer）
#[component]
pub fn AdminDashboardSkeleton() -> Element {
    rsx! {
        div { class: "space-y-8",
            // 统计卡片骨架
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                for _ in 0..3 {
                    div { class: "rounded-xl bg-white dark:bg-[#2e2e33] border border-gray-200 dark:border-[#333] p-6 text-center space-y-3",
                        SkeletonBox { class: "h-9 w-16 mx-auto rounded" }
                        SkeletonBox { class: "h-4 w-20 mx-auto rounded" }
                    }
                }
            }

            // 快捷操作骨架
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                SkeletonBox { class: "h-12 rounded-full" }
                SkeletonBox { class: "h-12 rounded-full" }
            }

            // 最近文章列表骨架
            div { class: "space-y-4",
                SkeletonBox { class: "h-6 w-24 rounded" }
                div { class: "space-y-0",
                    for _ in 0..5 {
                        div { class: "flex justify-between items-center py-3 border-b border-gray-100 dark:border-[#333]",
                            SkeletonBox { class: "h-4 w-[45%] rounded" }
                            SkeletonBox { class: "h-3 w-20 rounded" }
                        }
                    }
                }
            }
        }
    }
}

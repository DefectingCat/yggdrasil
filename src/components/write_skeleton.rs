use dioxus::prelude::*;
use crate::components::skeletons::atoms::*;

#[component]
pub fn WriteSkeleton() -> Element {
    rsx! {
        div { class: "space-y-4",
            // 标题输入骨架
            SkeletonBox { class: "w-full h-[52px] mb-4 rounded" }

            // 编辑器区域骨架
            div {
                class: "w-full h-[600px] border border-gray-200 dark:border-[#333] rounded-lg bg-white dark:bg-[#1e1e1e] p-6 space-y-4",
                // 工具栏骨架
                div { class: "flex gap-2 pb-4 border-b border-gray-100 dark:border-[#333]",
                    for _ in 0..8 {
                        SkeletonBox { class: "w-8 h-8 rounded" }
                    }
                }
                // 内容行骨架
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

            // 保存按钮骨架
            SkeletonBox { class: "mt-4 h-10 w-28 rounded-full" }
        }
    }
}

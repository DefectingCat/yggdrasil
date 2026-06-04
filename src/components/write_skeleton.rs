use dioxus::prelude::*;
use crate::components::skeletons::atoms::*;

#[component]
pub fn WriteSkeleton() -> Element {
    rsx! {
        div { class: "space-y-4",
            // 标题输入骨架
            SkeletonBox { class: "w-full h-[52px] mb-4".to_string() }

            // 编辑器区域骨架
            div {
                class: "w-full h-[600px] border border-gray-200 dark:border-[#333] rounded-lg bg-white dark:bg-[#1e1e1e] p-6 space-y-4",
                // 工具栏骨架
                div { class: "flex gap-2 pb-4 border-b border-gray-100 dark:border-[#333]",
                    for _ in 0..8 {
                        SkeletonBox { class: "w-8 h-8".to_string() }
                    }
                }
                // 内容行骨架
                div { class: "space-y-3 pt-2",
                    SkeletonBox { class: "h-4 w-[90%]".to_string() }
                    SkeletonBox { class: "h-4 w-full".to_string() }
                    SkeletonBox { class: "h-4 w-[85%]".to_string() }
                    SkeletonBox { class: "h-4 w-[95%]".to_string() }
                    SkeletonBox { class: "h-4 w-[60%]".to_string() }
                    SkeletonBox { class: "h-4 w-full".to_string() }
                    SkeletonBox { class: "h-4 w-[75%]".to_string() }
                    SkeletonBox { class: "h-4 w-[80%]".to_string() }
                    div { class: "h-4" }
                    SkeletonBox { class: "h-4 w-[70%]".to_string() }
                    SkeletonBox { class: "h-4 w-full".to_string() }
                    SkeletonBox { class: "h-4 w-[90%]".to_string() }
                }
            }

            // 保存按钮骨架
            SkeletonBox { class: "mt-4 h-10 w-28 rounded-full".to_string() }
        }
    }
}

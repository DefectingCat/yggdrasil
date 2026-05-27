use dioxus::prelude::*;

#[component]
pub fn WriteSkeleton() -> Element {
    rsx! {
        div { class: "space-y-4",
            // 标题输入骨架
            div { class: "w-full h-[52px] bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse mb-4" }

            // 编辑器区域骨架
            div {
                class: "w-full h-[600px] border border-gray-200 dark:border-[#333] rounded-lg bg-white dark:bg-[#1e1e1e] p-6 space-y-4",
                // 工具栏骨架
                div { class: "flex gap-2 pb-4 border-b border-gray-100 dark:border-[#333]",
                    for _ in 0..8 {
                        div { class: "w-8 h-8 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                    }
                }
                // 内容行骨架
                div { class: "space-y-3 pt-2",
                    div { class: "h-4 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse w-[90%]" }
                    div { class: "h-4 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse w-full" }
                    div { class: "h-4 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse w-[85%]" }
                    div { class: "h-4 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse w-[95%]" }
                    div { class: "h-4 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse w-[60%]" }
                    div { class: "h-4 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse w-full" }
                    div { class: "h-4 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse w-[75%]" }
                    div { class: "h-4 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse w-[80%]" }
                    div { class: "h-4" }
                    div { class: "h-4 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse w-[70%]" }
                    div { class: "h-4 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse w-full" }
                    div { class: "h-4 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse w-[90%]" }
                }
            }

            // 保存按钮骨架
            div { class: "mt-4 h-10 w-28 bg-gray-200 dark:bg-[#2a2a2a] rounded-full animate-pulse" }
        }
    }
}

use dioxus::prelude::*;

/// 文章卡片骨架屏 - 模拟 PostCard 的视觉结构
/// 包含：标题行(24px bold) + 摘要2行 + 元信息行(日期+标签)
#[component]
pub fn PostCardSkeleton() -> Element {
    rsx! {
        article {
            class: "mb-6 p-6 bg-white dark:bg-[#2e2e33] rounded-lg border border-gray-200 dark:border-[#333] animate-pulse",
            // 标题占位 (模拟 h2 text-2xl font-bold)
            div { class: "h-7 w-3/4 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-3" }
            // 摘要第一行
            div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded mb-2" }
            // 摘要第二行
            div { class: "h-4 w-5/6 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-3" }
            // 元信息行 (日期 + 标签)
            div { class: "flex items-center gap-3 mt-3",
                div { class: "h-3.5 w-20 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-3.5 w-1 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-3.5 w-16 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
            }
        }
    }
}

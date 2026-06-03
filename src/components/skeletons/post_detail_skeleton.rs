use dioxus::prelude::*;

/// 文章详情页骨架屏
/// 结构：面包屑 + 标题(h1) + 摘要 + 元信息 + 封面图 + 正文(多段) + Footer
#[component]
pub fn PostDetailSkeleton() -> Element {
    rsx! {
        article { class: "post-single",
            // 面包屑占位
            div { class: "h-4 w-48 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-6" }

            // 标题占位 (模拟 h1)
            div { class: "h-10 w-4/5 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-4" }

            // 摘要占位
            div { class: "h-5 w-2/3 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-4" }

            // 元信息行 (作者 · 日期 · 阅读时间)
            div { class: "flex items-center gap-2 mb-8",
                div { class: "h-4 w-16 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-4 w-1 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-4 w-24 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-4 w-1 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-4 w-20 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
            }

            // 封面图占位 (模拟 PostCover 16:9 比例)
            div { class: "w-full aspect-video bg-gray-200 dark:bg-[#2a2a2a] rounded-lg mb-8" }

            // 正文段落占位 (模拟多段 PostContent)
            div { class: "space-y-4 mb-8",
                div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-4 w-5/6 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-4 w-3/4 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                // 空行模拟段落间距
                div { class: "h-2" }
                div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-4 w-full bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                div { class: "h-4 w-2/3 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
            }

            // PostFooter 占位
            div { class: "border-t border-gray-200 dark:border-[#333] pt-6 mt-8",
                div { class: "flex items-center justify-between",
                    div { class: "h-4 w-32 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                    div { class: "h-4 w-24 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                }
            }
        }
    }
}
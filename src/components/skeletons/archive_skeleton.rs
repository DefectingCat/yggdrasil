use dioxus::prelude::*;

/// 归档页骨架屏
/// 结构：统计行("共 N 篇文章") + 年份标题 + 月份标题 + 文章条目列表
/// 模拟 2 个年份，每个年份 2 个月，每个月 3 篇文章
#[component]
pub fn ArchiveSkeleton() -> Element {
    rsx! {
        div { class: "animate-pulse",
            // 统计行占位
            div { class: "mt-2 mb-6",
                div { class: "h-5 w-32 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
            }

            // 年份分组占位
            for _ in 0..2 {
                div { class: "archive-year mt-10",
                    // 年份标题 (h2 text-2xl)
                    div { class: "h-8 w-24 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-4" }

                    // 月份分组
                    for _ in 0..2 {
                        div { class: "archive-month flex flex-col md:flex-row md:items-start py-2.5 border-b border-gray-100 dark:border-[#333]/50",
                            // 月份标题 (h3 text-lg, md:w-[200px])
                            div { class: "h-6 w-32 md:w-[200px] shrink-0 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-2 md:mb-0 md:py-1.5" }

                            // 文章条目列表
                            div { class: "flex-1 space-y-3",
                                for _ in 0..3 {
                                    div { class: "archive-entry py-1.5 my-2.5",
                                        // 文章标题
                                        div { class: "h-4 w-3/4 bg-gray-200 dark:bg-[#2a2a2a] rounded mb-1" }
                                        // 日期
                                        div { class: "h-3 w-20 bg-gray-200 dark:bg-[#2a2a2a] rounded" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

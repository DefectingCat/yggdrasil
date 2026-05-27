use dioxus::prelude::*;

/// 仅仪表盘内容区骨架（不含 header/footer）
#[component]
pub fn AdminDashboardSkeleton() -> Element {
    rsx! {
        div { class: "space-y-8",
            // 统计卡片骨架
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                for _ in 0..3 {
                    div { class: "rounded-xl bg-white dark:bg-[#2e2e33] border border-gray-200 dark:border-[#333] p-6 text-center space-y-3",
                        div { class: "h-9 w-16 mx-auto bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                        div { class: "h-4 w-20 mx-auto bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                    }
                }
            }

            // 快捷操作骨架
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                div { class: "h-12 bg-gray-200 dark:bg-[#2a2a2a] rounded-full animate-pulse" }
                div { class: "h-12 bg-gray-200 dark:bg-[#2a2a2a] rounded-full animate-pulse" }
            }

            // 最近文章列表骨架
            div { class: "space-y-4",
                div { class: "h-6 w-24 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                div { class: "space-y-0",
                    for _ in 0..5 {
                        div { class: "flex justify-between items-center py-3 border-b border-gray-100 dark:border-[#333]",
                            div { class: "h-4 w-[45%] bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                            div { class: "h-3 w-20 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                        }
                    }
                }
            }
        }
    }
}

/// 完整的仪表盘页面骨架（含 header/footer + 内容）
#[component]
pub fn AdminSkeleton() -> Element {
    rsx! {
        div { class: "min-h-screen flex flex-col bg-white dark:bg-[#1d1e20]",
            // Header 骨架
            header { class: "sticky top-0 z-40 w-full border-b border-gray-200 dark:border-[#333] bg-white/80 dark:bg-[#1d1e20]/80 backdrop-blur-sm",
                nav { class: "max-w-3xl mx-auto px-6 h-[60px] flex items-center justify-between",
                    // Logo 占位
                    div { class: "w-32 h-7 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                    // 导航项 + 右侧按钮占位
                    div { class: "flex items-center gap-4",
                        div { class: "hidden md:flex items-center gap-2",
                            div { class: "w-12 h-5 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                            div { class: "w-12 h-5 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                            div { class: "w-10 h-5 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                        }
                        div { class: "w-10 h-5 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                    }
                }
            }

            // 内容区骨架
            main { class: "flex-1 w-full max-w-5xl mx-auto px-6 py-8",
                AdminDashboardSkeleton {}
            }

            // Footer 骨架
            footer { class: "w-full border-t border-gray-200 dark:border-[#333] py-6",
                div { class: "max-w-3xl mx-auto px-6 flex justify-between items-center",
                    div { class: "h-4 w-32 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                    div { class: "h-4 w-24 bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse" }
                }
            }
        }
    }
}

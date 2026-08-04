//! 后台系统管理骨架屏
//!
//! 镜像后台 System 页面的结构：Header（标题+副标题）+ 5 个功能 Tabs + 4 个统计卡片 + 数据行列表。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::ui::{ADMIN_CARD_CLASS, ADMIN_TABLE_CLASS};

/// 后台系统管理骨架屏组件。
#[component]
pub fn SystemSkeleton() -> Element {
    rsx! {
        div { class: "w-full max-w-7xl mx-auto space-y-6",
            // 页头：标题与副标题
            div { class: "pb-6 border-b border-[var(--color-paper-border)] mb-6",
                div { class: "space-y-2",
                    SkeletonBox { class: "h-9 w-36 rounded-lg" }
                    SkeletonBox { class: "h-4 w-48 rounded" }
                }
            }

            // 5 个功能 Tabs
            div { class: "flex gap-2 border-b border-paper-border pb-2 mb-6",
                SkeletonBox { class: "h-8 w-24 rounded-full" }
                SkeletonBox { class: "h-8 w-24 rounded-full" }
                SkeletonBox { class: "h-8 w-24 rounded-full" }
                SkeletonBox { class: "h-8 w-20 rounded-full" }
                SkeletonBox { class: "h-8 w-20 rounded-full" }
            }

            // 4 个统计卡片网格
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4",
                for _ in 0..4 {
                    div { class: "{ADMIN_CARD_CLASS} p-6 space-y-3 text-center",
                        SkeletonBox { class: "h-3 w-16 mx-auto rounded" }
                        SkeletonBox { class: "h-8 w-24 mx-auto rounded-lg" }
                    }
                }
            }

            // 数据表格卡片占位
            div { class: "{ADMIN_TABLE_CLASS}",
                table { class: "w-full text-sm",
                    thead {
                        tr { class: "border-b border-paper-border",
                            th { class: "px-4 py-3", SkeletonBox { class: "h-3 w-16" } }
                            th { class: "px-4 py-3", SkeletonBox { class: "h-3 w-20" } }
                            th { class: "px-4 py-3", SkeletonBox { class: "h-3 w-24" } }
                            th { class: "px-4 py-3 w-20", SkeletonBox { class: "h-3 w-12 ml-auto" } }
                        }
                    }
                    tbody {
                        for _ in 0..5 {
                            tr { class: "border-b border-paper-border last:border-0",
                                td { class: "px-4 py-3", SkeletonBox { class: "h-4 w-28" } }
                                td { class: "px-4 py-3", SkeletonBox { class: "h-4 w-36" } }
                                td { class: "px-4 py-3", SkeletonBox { class: "h-4 w-40" } }
                                td { class: "px-4 py-3", SkeletonBox { class: "h-4 w-16 ml-auto" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

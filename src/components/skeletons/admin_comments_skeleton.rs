//! 后台评论管理骨架屏
//!
//! 镜像后台 AdminComments 页面的结构：Header（标题+描述）+ 状态筛选 Tabs + 5 条评论卡片行。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::ui::ADMIN_CARD_CLASS;

/// 后台评论管理骨架屏组件。
#[component]
pub fn AdminCommentsSkeleton() -> Element {
    rsx! {
        div { class: "w-full max-w-7xl mx-auto space-y-6",
            // 页头：标题与描述
            div { class: "pb-6 border-b border-paper-border mb-6",
                div { class: "space-y-2",
                    SkeletonBox { class: "h-9 w-36 rounded-lg" }
                    SkeletonBox { class: "h-4 w-48 rounded" }
                }
            }

            // 状态筛选 Tabs
            div { class: "flex gap-2 border-b border-paper-border pb-2 mb-6",
                SkeletonBox { class: "h-8 w-16 rounded-full" }
                SkeletonBox { class: "h-8 w-20 rounded-full" }
                SkeletonBox { class: "h-8 w-20 rounded-full" }
                SkeletonBox { class: "h-8 w-20 rounded-full" }
            }

            // 评论列表卡片
            div { class: "{ADMIN_CARD_CLASS} p-6 space-y-4",
                for _ in 0..5 {
                    div { class: "flex items-center gap-4 py-2 border-b border-paper-border last:border-0",
                        SkeletonBox { class: "h-4 w-4 rounded" }
                        SkeletonBox { class: "h-9 w-9 rounded-full flex-shrink-0" }
                        div { class: "flex-1 space-y-2 min-w-0",
                            div { class: "flex items-center gap-2",
                                SkeletonBox { class: "h-4 w-28 rounded" }
                                SkeletonBox { class: "h-3 w-20 rounded" }
                                SkeletonBox { class: "h-4 w-12 rounded-full" }
                            }
                            SkeletonBox { class: "h-4 w-3/4 rounded" }
                        }
                        div { class: "flex gap-2 flex-shrink-0",
                            SkeletonBox { class: "h-8 w-14 rounded-full" }
                            SkeletonBox { class: "h-8 w-14 rounded-full" }
                        }
                    }
                }
            }
        }
    }
}

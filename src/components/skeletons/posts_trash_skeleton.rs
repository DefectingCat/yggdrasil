//! 后台回收站骨架屏
//!
//! 镜像后台 PostsTrash 页面的结构：Header（标题+副标题）+ 自动清理配置卡片 + 表格。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::ui::{ADMIN_CARD_CLASS, ADMIN_TABLE_CLASS};

/// 后台回收站骨架屏组件。
#[component]
pub fn PostsTrashSkeleton() -> Element {
    rsx! {
        div { class: "w-full max-w-7xl mx-auto space-y-6",
            // 页头：标题与副标题
            div { class: "pb-6 border-b border-paper-border mb-6",
                div { class: "space-y-2",
                    SkeletonBox { class: "h-9 w-28 rounded-lg" }
                    SkeletonBox { class: "h-4 w-36 rounded" }
                }
            }

            div { class: "space-y-6",
                // 自动清理配置卡片占位
                div { class: "{ADMIN_CARD_CLASS} p-6 space-y-4",
                    div { class: "flex justify-between items-center",
                        div { class: "space-y-1",
                            SkeletonBox { class: "h-5 w-32 rounded" }
                            SkeletonBox { class: "h-3.5 w-64 rounded" }
                        }
                        SkeletonBox { class: "h-6 w-12 rounded-full" }
                    }
                }

                // 回收站表格
                div { class: "{ADMIN_TABLE_CLASS}",
                    table { class: "w-full text-sm",
                        thead {
                            tr { class: "border-b border-paper-border",
                                th { class: "px-4 py-3 w-10", SkeletonBox { class: "h-4 w-4 rounded" } }
                                th { class: "px-4 py-3", SkeletonBox { class: "h-3 w-12" } }
                                th { class: "px-4 py-3 w-24", SkeletonBox { class: "h-3 w-10 mx-auto" } }
                                th { class: "px-4 py-3 w-32", SkeletonBox { class: "h-3 w-14" } }
                                th { class: "px-4 py-3 w-24", SkeletonBox { class: "h-3 w-14 mx-auto" } }
                                th { class: "px-4 py-3 w-28", SkeletonBox { class: "h-3 w-12 ml-auto" } }
                            }
                        }
                        tbody {
                            for _ in 0..8 {
                                tr { class: "border-b border-paper-border last:border-0",
                                    td { class: "px-4 py-3", SkeletonBox { class: "h-4 w-4 rounded" } }
                                    td { class: "px-4 py-3", SkeletonBox { class: "h-4 w-1/3" } }
                                    td { class: "px-4 py-3", SkeletonBox { class: "h-5 w-14 mx-auto rounded" } }
                                    td { class: "px-4 py-3", SkeletonBox { class: "h-4 w-20" } }
                                    td { class: "px-4 py-3", SkeletonBox { class: "h-5 w-16 mx-auto rounded" } }
                                    td { class: "px-4 py-3", SkeletonBox { class: "h-4 w-20 ml-auto" } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

//! 后台友链管理骨架屏
//!
//! 镜像后台 FriendsAdmin 页面的结构：Header（标题+描述）+ 表单卡片 + 列表卡片。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::ui::{ADMIN_CARD_CLASS, ADMIN_TABLE_CLASS};

/// 后台友链管理骨架屏组件。
#[component]
pub fn FriendsAdminSkeleton() -> Element {
    rsx! {
        div { class: "w-full max-w-7xl mx-auto space-y-8",
            // 页头
            div { class: "flex flex-col md:flex-row md:items-end justify-between gap-6 pb-8 border-b border-[var(--color-paper-border)]/50",
                div { class: "space-y-2",
                    SkeletonBox { class: "h-9 w-36 rounded-lg" }
                    SkeletonBox { class: "h-4 w-60 rounded" }
                }
            }

            // 表单卡片占位
            div { class: "{ADMIN_CARD_CLASS} p-8 space-y-6",
                SkeletonBox { class: "h-6 w-28 rounded" }
                div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                    div { class: "space-y-2",
                        SkeletonBox { class: "h-4 w-16 rounded" }
                        SkeletonBox { class: "h-10 w-full rounded-2xl" }
                    }
                    div { class: "space-y-2",
                        SkeletonBox { class: "h-4 w-16 rounded" }
                        SkeletonBox { class: "h-10 w-full rounded-2xl" }
                    }
                    div { class: "space-y-2",
                        SkeletonBox { class: "h-4 w-24 rounded" }
                        SkeletonBox { class: "h-10 w-full rounded-2xl" }
                    }
                    div { class: "space-y-2",
                        SkeletonBox { class: "h-4 w-20 rounded" }
                        SkeletonBox { class: "h-10 w-full rounded-2xl" }
                    }
                    div { class: "space-y-2 md:col-span-2",
                        SkeletonBox { class: "h-4 w-16 rounded" }
                        SkeletonBox { class: "h-16 w-full rounded-2xl" }
                    }
                }
                SkeletonBox { class: "h-10 w-24 rounded-full" }
            }

            // 友链列表表格占位
            div { class: "{ADMIN_TABLE_CLASS}",
                table { class: "w-full text-sm",
                    thead {
                        tr { class: "border-b border-paper-border",
                            th { class: "px-4 py-3 w-16",
                                SkeletonBox { class: "h-3 w-8" }
                            }
                            th { class: "px-4 py-3",
                                SkeletonBox { class: "h-3 w-12" }
                            }
                            th { class: "px-4 py-3",
                                SkeletonBox { class: "h-3 w-16" }
                            }
                            th { class: "px-4 py-3 w-20",
                                SkeletonBox { class: "h-3 w-10 mx-auto" }
                            }
                            th { class: "px-4 py-3 w-32",
                                SkeletonBox { class: "h-3 w-12 ml-auto" }
                            }
                        }
                    }
                    tbody {
                        for _ in 0..5 {
                            tr { class: "border-b border-paper-border last:border-0",
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-4 w-6" }
                                }
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-4 w-24" }
                                }
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-4 w-40" }
                                }
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-5 w-12 mx-auto rounded" }
                                }
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-4 w-20 ml-auto" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

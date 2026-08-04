//! 后台文章管理列表骨架屏
//!
//! 镜像后台 Posts 页面的结构：Header（标题+按钮）+ 搜索栏 + 表格 + 分页栏。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::ui::ADMIN_TABLE_CLASS;

/// 后台文章管理列表骨架屏组件。
#[component]
pub fn PostsSkeleton() -> Element {
    rsx! {
        div { class: "w-full max-w-7xl mx-auto space-y-6",
            // 页头：标题 + 操作按钮
            div { class: "flex flex-col md:flex-row md:items-end justify-between gap-6 pb-6 border-b border-paper-border mb-6",
                div { class: "space-y-2",
                    SkeletonBox { class: "h-9 w-36 rounded-lg" }
                    SkeletonBox { class: "h-4 w-48 rounded" }
                }
                div { class: "flex items-center gap-3",
                    SkeletonBox { class: "h-10 w-28 rounded-full" }
                    SkeletonBox { class: "h-10 w-24 rounded-full" }
                }
            }

            // 搜索/筛选工具栏
            div { class: "flex gap-2 mb-4",
                SkeletonBox { class: "h-10 flex-1 rounded-2xl" }
                SkeletonBox { class: "h-10 w-20 rounded-full" }
            }

            // 文章列表表格
            div { class: "{ADMIN_TABLE_CLASS}",
                table { class: "w-full text-sm",
                    thead {
                        tr { class: "border-b border-paper-border",
                            th { class: "px-4 py-3",
                                SkeletonBox { class: "h-3 w-10" }
                            }
                            th { class: "px-4 py-3 w-24",
                                SkeletonBox { class: "h-3 w-10 mx-auto" }
                            }
                            th { class: "px-4 py-3 w-32",
                                SkeletonBox { class: "h-3 w-10" }
                            }
                            th { class: "px-4 py-3 w-24",
                                SkeletonBox { class: "h-3 w-10 ml-auto" }
                            }
                        }
                    }
                    tbody {
                        for _ in 0..10 {
                            tr { class: "border-b border-paper-border last:border-0",
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-4 w-1/3" }
                                }
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-5 w-14 mx-auto rounded" }
                                }
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-4 w-20" }
                                }
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-4 w-12 ml-auto" }
                                }
                            }
                        }
                    }
                }
            }

            // 分页栏
            div { class: "flex justify-between items-center pt-4 border-t border-paper-border",
                SkeletonBox { class: "h-4 w-32 rounded" }
                div { class: "flex gap-2",
                    SkeletonBox { class: "h-8 w-16 rounded-full" }
                    SkeletonBox { class: "h-8 w-16 rounded-full" }
                }
            }
        }
    }
}

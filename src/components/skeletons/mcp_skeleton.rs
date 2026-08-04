//! 后台 MCP 服务骨架屏
//!
//! 镜像后台 Mcp 页面的结构：Header（标题+描述）+ Token 列表表格卡片 + 新建 Token 表单卡片 + 客户端配置网格。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::ui::{ADMIN_CARD_CLASS, ADMIN_TABLE_CLASS};

/// 后台 MCP 服务骨架屏组件。
#[component]
pub fn McpSkeleton() -> Element {
    rsx! {
        div { class: "w-full max-w-7xl mx-auto space-y-8",
            // 页头
            div { class: "flex flex-col md:flex-row md:items-end justify-between gap-6 pb-8 border-b border-[var(--color-paper-border)]/50",
                div { class: "space-y-2",
                    SkeletonBox { class: "h-9 w-36 rounded-lg" }
                    SkeletonBox { class: "h-4 w-96 rounded" }
                }
            }

            // Token 列表表格卡片
            div { class: "{ADMIN_TABLE_CLASS}",
                table { class: "w-full text-sm",
                    thead {
                        tr { class: "border-b border-paper-border",
                            th { class: "px-4 py-3",
                                SkeletonBox { class: "h-3 w-16" }
                            }
                            th { class: "px-4 py-3 w-24",
                                SkeletonBox { class: "h-3 w-12 mx-auto" }
                            }
                            th { class: "px-4 py-3 w-32",
                                SkeletonBox { class: "h-3 w-16" }
                            }
                            th { class: "px-4 py-3 w-32",
                                SkeletonBox { class: "h-3 w-16" }
                            }
                            th { class: "px-4 py-3 w-24",
                                SkeletonBox { class: "h-3 w-12 ml-auto" }
                            }
                        }
                    }
                    tbody {
                        for _ in 0..4 {
                            tr { class: "border-b border-paper-border last:border-0",
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-4 w-28" }
                                }
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-5 w-14 mx-auto rounded" }
                                }
                                td { class: "px-4 py-3",
                                    SkeletonBox { class: "h-4 w-20" }
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

            // 新建 Token 表单卡片占位
            div { class: "{ADMIN_CARD_CLASS} p-8 space-y-6",
                SkeletonBox { class: "h-6 w-32 rounded" }
                div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                    div { class: "space-y-2",
                        SkeletonBox { class: "h-4 w-16 rounded" }
                        SkeletonBox { class: "h-10 w-full rounded-2xl" }
                    }
                    div { class: "space-y-2",
                        SkeletonBox { class: "h-4 w-16 rounded" }
                        SkeletonBox { class: "h-10 w-full rounded-2xl" }
                    }
                    div { class: "space-y-2",
                        SkeletonBox { class: "h-4 w-16 rounded" }
                        SkeletonBox { class: "h-10 w-full rounded-2xl" }
                    }
                }
                SkeletonBox { class: "h-10 w-28 rounded-full" }
            }

            // 客户端配置卡片占位
            div { class: "{ADMIN_CARD_CLASS} p-8 space-y-6",
                SkeletonBox { class: "h-6 w-36 rounded" }
                div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                    for _ in 0..4 {
                        div { class: "p-4 border border-paper-border rounded-2xl space-y-3",
                            SkeletonBox { class: "h-5 w-28 rounded" }
                            SkeletonBox { class: "h-24 w-full rounded-xl" }
                        }
                    }
                }
            }
        }
    }
}

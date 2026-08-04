//! 后台代码试运行沙箱骨架屏
//!
//! 镜像后台 Runner 页面的结构：Header（标题+描述）+ 语言切换 Pills + 沙箱配置/代码编辑器 + 输出面板。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::ui::ADMIN_CARD_CLASS;

/// 后台代码试运行沙箱骨架屏组件。
#[component]
pub fn RunnerSkeleton() -> Element {
    rsx! {
        div { class: "w-full max-w-7xl mx-auto space-y-8",
            // 页头
            div { class: "flex flex-col md:flex-row md:items-end justify-between gap-6 pb-8 border-b border-[var(--color-paper-border)]/50",
                div { class: "space-y-2",
                    SkeletonBox { class: "h-9 w-48 rounded-lg" }
                    SkeletonBox { class: "h-4 w-96 rounded" }
                }
            }

            // 配置卡片：语言切换 Tabs
            div { class: "{ADMIN_CARD_CLASS} p-8 flex flex-col gap-6",
                div { class: "flex flex-col gap-2",
                    SkeletonBox { class: "h-4 w-12 rounded" }
                    div { class: "flex gap-2",
                        SkeletonBox { class: "h-8 w-20 rounded-full" }
                        SkeletonBox { class: "h-8 w-16 rounded-full" }
                        SkeletonBox { class: "h-8 w-14 rounded-full" }
                        SkeletonBox { class: "h-8 w-16 rounded-full" }
                        SkeletonBox { class: "h-8 w-16 rounded-full" }
                    }
                }
            }

            // 沙箱代码编辑器卡片占位
            div { class: "{ADMIN_CARD_CLASS} p-6 space-y-4",
                div { class: "flex justify-between items-center pb-3 border-b border-paper-border",
                    SkeletonBox { class: "h-5 w-24 rounded" }
                    SkeletonBox { class: "h-9 w-24 rounded-full" }
                }
                SkeletonBox { class: "h-64 w-full rounded-2xl" }
            }

            // 执行输出面板占位
            div { class: "{ADMIN_CARD_CLASS} p-6 space-y-3",
                SkeletonBox { class: "h-5 w-20 rounded" }
                SkeletonBox { class: "h-32 w-full rounded-2xl" }
            }
        }
    }
}

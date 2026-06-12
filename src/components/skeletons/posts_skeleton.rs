//! 后台文章管理列表骨架屏
//!
//! 模拟后台 Posts 页面的表格结构。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;

/// 后台文章管理列表骨架屏组件。
///
/// 渲染带表头的表格占位，包含 10 行数据行。
#[component]
pub fn PostsSkeleton() -> Element {
    rsx! {
        div { class: "bg-white dark:bg-[#2e2e33] rounded-xl border border-gray-200 dark:border-[#333] overflow-hidden",
            table { class: "w-full text-sm",
                thead {
                    tr { class: "border-b border-gray-200 dark:border-[#333]",
                        th { class: "px-4 py-3", SkeletonBox { class: "h-3 w-10" } }
                        th { class: "px-4 py-3 w-24", SkeletonBox { class: "h-3 w-10 mx-auto" } }
                        th { class: "px-4 py-3 w-32", SkeletonBox { class: "h-3 w-10" } }
                        th { class: "px-4 py-3 w-24", SkeletonBox { class: "h-3 w-10 ml-auto" } }
                    }
                }
                tbody {
                    for _ in 0..10 {
                        tr { class: "border-b border-gray-100 dark:border-[#333] last:border-0",
                            td { class: "px-4 py-3", SkeletonBox { class: "h-4 w-1/3" } }
                            td { class: "px-4 py-3", SkeletonBox { class: "h-5 w-14 mx-auto rounded" } }
                            td { class: "px-4 py-3", SkeletonBox { class: "h-4 w-20" } }
                            td { class: "px-4 py-3", SkeletonBox { class: "h-4 w-12 ml-auto" } }
                        }
                    }
                }
            }
        }
    }
}

use dioxus::prelude::*;
use crate::components::skeletons::atoms::*;

/// 文章详情页骨架屏
/// 结构：面包屑 + 标题(h1) + 摘要 + 元信息 + 封面图 + 正文(多段) + Footer
#[component]
pub fn PostDetailSkeleton() -> Element {
    rsx! {
        article { class: "post-single",
            // 面包屑占位
            SkeletonBox { class: "h-4 w-48 mb-6".to_string() }

            // 标题占位 (模拟 h1)
            SkeletonBox { class: "h-10 w-4/5 mb-4".to_string() }

            // 摘要占位
            SkeletonBox { class: "h-5 w-2/3 mb-4".to_string() }

            // 元信息行 (作者 · 日期 · 阅读时间)
            div { class: "flex items-center gap-2 mb-8",
                SkeletonBox { class: "h-4 w-16".to_string() }
                SkeletonBox { class: "h-4 w-1".to_string() }
                SkeletonBox { class: "h-4 w-24".to_string() }
                SkeletonBox { class: "h-4 w-1".to_string() }
                SkeletonBox { class: "h-4 w-20".to_string() }
            }

            // 封面图占位 (模拟 PostCover 16:9 比例)
            SkeletonBox { class: "w-full aspect-video rounded-lg mb-8".to_string() }

            // 正文段落占位 (模拟多段 PostContent)
            div { class: "space-y-4 mb-8",
                SkeletonBox { class: "h-4 w-full".to_string() }
                SkeletonBox { class: "h-4 w-full".to_string() }
                SkeletonBox { class: "h-4 w-5/6".to_string() }
                SkeletonBox { class: "h-4 w-full".to_string() }
                SkeletonBox { class: "h-4 w-full".to_string() }
                SkeletonBox { class: "h-4 w-3/4".to_string() }
                // 空行模拟段落间距
                div { class: "h-2" }
                SkeletonBox { class: "h-4 w-full".to_string() }
                SkeletonBox { class: "h-4 w-full".to_string() }
                SkeletonBox { class: "h-4 w-2/3".to_string() }
            }

            // PostFooter 占位
            div { class: "border-t border-gray-200 dark:border-[#333] pt-6 mt-8",
                div { class: "flex items-center justify-between",
                    SkeletonBox { class: "h-4 w-32".to_string() }
                    SkeletonBox { class: "h-4 w-24".to_string() }
                }
            }
        }
    }
}

//! 首页骨架屏
//!
//! 模拟首页文章卡片列表与分页区域。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::skeletons::post_card_skeleton::PostCardSkeleton;

/// 首页骨架屏组件。
///
/// 显示与 `POSTS_PER_PAGE` 等量的文章卡片骨架与分页按钮占位，
/// 使骨架屏与加载完成后的实际列表长度一致，避免内容跳变。
#[component]
pub fn HomeSkeleton() -> Element {
    rsx! {
        div {
            // 10 个文章卡片骨架，对齐首页 POSTS_PER_PAGE。
            for _ in 0..10 {
                PostCardSkeleton {}
            }
            // 分页按钮占位
            div { class: "flex mt-10 mb-6 justify-between",
                SkeletonBox { class: "h-9 w-24 rounded-full" }
                SkeletonBox { class: "h-9 w-24 rounded-full" }
            }
        }
    }
}

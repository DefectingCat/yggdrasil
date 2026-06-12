//! 首页骨架屏
//!
//! 模拟首页文章卡片列表与分页区域。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::skeletons::post_card_skeleton::PostCardSkeleton;

/// 首页骨架屏组件。
///
/// 显示 5 个文章卡片骨架与分页按钮占位。
#[component]
pub fn HomeSkeleton() -> Element {
    rsx! {
        div {
            // 5 个文章卡片骨架
            for _ in 0..5 {
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

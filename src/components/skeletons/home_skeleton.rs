use dioxus::prelude::*;

use crate::components::skeletons::post_card_skeleton::PostCardSkeleton;

/// 首页骨架屏 - 模拟文章卡片列表 + 分页区域
/// 显示 5 个文章卡片骨架 + 分页按钮占位
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
                div { class: "h-9 w-24 bg-gray-200 dark:bg-[#2a2a2a] rounded-full" }
                div { class: "h-9 w-24 bg-gray-200 dark:bg-[#2a2a2a] rounded-full" }
            }
        }
    }
}

use dioxus::prelude::*;

use crate::components::skeletons::post_card_skeleton::PostCardSkeleton;

/// 搜索页骨架屏 - 搜索结果卡片列表
/// 模拟 3 个搜索结果卡片（与搜索页现有内联骨架结构一致）
#[component]
pub fn SearchSkeleton() -> Element {
    rsx! {
        div { class: "space-y-6 py-4 animate-pulse",
            // 3 个结果卡片骨架
            for _ in 0..3 {
                PostCardSkeleton {}
            }
        }
    }
}

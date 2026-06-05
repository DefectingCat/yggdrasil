use dioxus::prelude::*;
use crate::components::skeletons::atoms::*;

#[component]
pub fn WriteSkeleton() -> Element {
    rsx! {
        div { class: "space-y-6 p-1",
            div { class: "rounded-xl bg-white dark:bg-[#2e2e33] border border-gray-200 dark:border-[#333] p-6 space-y-5",
                SkeletonBox { class: "h-9 w-2/3 rounded-lg" }
                SkeletonBox { class: "h-16 w-full rounded-lg" }
                div { class: "grid grid-cols-1 md:grid-cols-3 gap-3",
                    for _ in 0..3 {
                        div { class: "space-y-2",
                            SkeletonBox { class: "h-3 w-10 rounded" }
                            SkeletonBox { class: "h-10 w-full rounded-lg" }
                        }
                    }
                }
            }

            div { class: "w-full h-[500px] rounded-lg border border-gray-200 dark:border-[#333] bg-white dark:bg-[#1e1e1e] p-6 space-y-4",
                div { class: "flex gap-2 pb-4 border-b border-gray-100 dark:border-[#333]",
                    for _ in 0..8 {
                        SkeletonBox { class: "w-8 h-8 rounded" }
                    }
                }
                div { class: "space-y-3 pt-2",
                    SkeletonBox { class: "h-4 w-[90%] rounded" }
                    SkeletonBox { class: "h-4 w-full rounded" }
                    SkeletonBox { class: "h-4 w-[85%] rounded" }
                    SkeletonBox { class: "h-4 w-[95%] rounded" }
                    SkeletonBox { class: "h-4 w-[60%] rounded" }
                    SkeletonBox { class: "h-4 w-full rounded" }
                    SkeletonBox { class: "h-4 w-[75%] rounded" }
                    SkeletonBox { class: "h-4 w-[80%] rounded" }
                    div { class: "h-4" }
                    SkeletonBox { class: "h-4 w-[70%] rounded" }
                    SkeletonBox { class: "h-4 w-full rounded" }
                    SkeletonBox { class: "h-4 w-[90%] rounded" }
                }
            }

            div { class: "flex justify-end gap-3 pt-2",
                SkeletonBox { class: "h-10 w-20 rounded-full" }
                SkeletonBox { class: "h-10 w-24 rounded-full" }
                SkeletonBox { class: "h-10 w-20 rounded-full" }
            }
        }
    }
}

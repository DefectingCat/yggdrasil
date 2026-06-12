use crate::components::skeletons::atoms::*;
use dioxus::prelude::*;

#[component]
pub fn CommentListSkeleton() -> Element {
    rsx! {
        div { class: "animate-pulse space-y-6",
            div { class: "h-8 w-32 bg-paper-tertiary/30 rounded mb-6" }
            div { class: "space-y-4 bg-paper-tertiary/30 rounded-lg p-4",
                div { class: "flex gap-3",
                    div { class: "w-10 h-10 rounded-full bg-paper-tertiary/50 shrink-0" }
                    div { class: "flex-1 space-y-2",
                        SkeletonBox { class: "h-4 w-1/4 rounded" }
                        SkeletonBox { class: "h-3 w-3/4 rounded" }
                    }
                }
            }
            div { class: "space-y-4 bg-paper-tertiary/30 rounded-lg p-4 ml-6",
                div { class: "flex gap-3",
                    div { class: "w-10 h-10 rounded-full bg-paper-tertiary/50 shrink-0" }
                    div { class: "flex-1 space-y-2",
                        SkeletonBox { class: "h-4 w-1/4 rounded" }
                        SkeletonBox { class: "h-3 w-3/4 rounded" }
                    }
                }
            }
            div { class: "space-y-4 bg-paper-tertiary/30 rounded-lg p-4",
                div { class: "flex gap-3",
                    div { class: "w-10 h-10 rounded-full bg-paper-tertiary/50 shrink-0" }
                    div { class: "flex-1 space-y-2",
                        SkeletonBox { class: "h-4 w-1/4 rounded" }
                        SkeletonBox { class: "h-3 w-3/4 rounded" }
                    }
                }
            }
        }
    }
}

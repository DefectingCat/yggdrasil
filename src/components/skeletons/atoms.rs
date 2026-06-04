use dioxus::prelude::*;

#[component]
pub fn SkeletonLine(width: String, height: String) -> Element {
    rsx! {
        div {
            class: "bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse",
            style: "width: {width}; height: {height};",
        }
    }
}

#[component]
pub fn SkeletonBox(class: String) -> Element {
    rsx! {
        div {
            class: "bg-gray-200 dark:bg-[#2a2a2a] rounded animate-pulse {class}",
        }
    }
}

#[component]
pub fn SkeletonCard() -> Element {
    rsx! {
        div { class: "rounded-xl bg-white dark:bg-[#2e2e33] border border-gray-200 dark:border-[#333] p-6 text-center space-y-3",
            SkeletonLine { width: "64px".to_string(), height: "36px".to_string() }
            SkeletonLine { width: "80px".to_string(), height: "16px".to_string() }
        }
    }
}

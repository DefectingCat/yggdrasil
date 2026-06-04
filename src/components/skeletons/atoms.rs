use dioxus::prelude::*;

#[component]
pub fn SkeletonBox(class: &'static str, style: Option<&'static str>) -> Element {
    rsx! {
        div {
            class: "bg-gray-200 dark:bg-[#2a2a2a] animate-pulse {class}",
            style: style.unwrap_or(""),
        }
    }
}

use dioxus::prelude::*;

#[component]
pub fn SkeletonBox(class: &'static str, style: Option<&'static str>) -> Element {
    rsx! {
        div {
            class: "bg-paper-tertiary/30 dark:bg-[#5a5a62] animate-pulse {class}",
            style: style.unwrap_or(""),
        }
    }
}

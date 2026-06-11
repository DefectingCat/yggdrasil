use dioxus::prelude::*;

#[component]
pub fn SkeletonBox(class: &'static str, style: Option<&'static str>) -> Element {
    rsx! {
        div {
            class: "bg-paper-tertiary/30 animate-pulse {class}",
            style: style.unwrap_or(""),
        }
    }
}

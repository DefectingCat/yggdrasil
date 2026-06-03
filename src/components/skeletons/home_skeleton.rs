use dioxus::prelude::*;

#[component]
pub fn HomeSkeleton() -> Element {
    rsx! {
        div { class: "animate-pulse", "Loading..." }
    }
}

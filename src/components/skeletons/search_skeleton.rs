use dioxus::prelude::*;

#[component]
pub fn SearchSkeleton() -> Element {
    rsx! {
        div { class: "animate-pulse", "Loading..." }
    }
}

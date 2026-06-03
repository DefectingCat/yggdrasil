use dioxus::prelude::*;

#[component]
pub fn TagsSkeleton() -> Element {
    rsx! {
        div { class: "animate-pulse", "Loading..." }
    }
}

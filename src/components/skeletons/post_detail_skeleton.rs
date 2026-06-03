use dioxus::prelude::*;

#[component]
pub fn PostDetailSkeleton() -> Element {
    rsx! {
        div { class: "animate-pulse", "Loading..." }
    }
}

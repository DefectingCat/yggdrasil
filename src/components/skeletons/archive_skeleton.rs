use dioxus::prelude::*;

#[component]
pub fn ArchiveSkeleton() -> Element {
    rsx! {
        div { class: "animate-pulse", "Loading..." }
    }
}

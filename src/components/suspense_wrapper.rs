use dioxus::prelude::*;

/// Wraps children in a SuspenseBoundary with a loading skeleton fallback.
/// Used for pages that fetch data via `use_server_future`.
#[component]
pub fn SuspenseWrapper(children: Element) -> Element {
    rsx! {
        SuspenseBoundary {
            fallback: |_| rsx! {
                div { class: "animate-pulse py-6 space-y-4",
                    div { class: "h-10 w-3/4 bg-paper-tertiary rounded" }
                    div { class: "h-4 w-32 bg-paper-tertiary rounded" }
                    div { class: "h-4 w-full bg-paper-tertiary rounded mt-8" }
                    div { class: "h-4 w-full bg-paper-tertiary rounded" }
                    div { class: "h-4 w-2/3 bg-paper-tertiary rounded" }
                }
            },
            {children}
        }
    }
}

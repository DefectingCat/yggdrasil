use dioxus::prelude::*;

#[component]
pub fn PostToc(toc_html: String) -> Element {
    rsx! {
        details { class: "toc",
            summary {
                accesskey: "c",
                title: "(Alt + C)",
                span { class: "title", "Table of Contents" }
            }
            div {
                class: "inner",
                dangerous_inner_html: "{toc_html}"
            }
        }
    }
}

use dioxus::prelude::*;

#[component]
pub fn PostCover(src: String) -> Element {
    rsx! {
        figure { class: "entry-cover",
            img {
                loading: "eager",
                src: "{src}",
                alt: "Cover image"
            }
        }
    }
}

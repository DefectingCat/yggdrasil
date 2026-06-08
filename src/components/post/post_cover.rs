use dioxus::prelude::*;

use crate::components::image_viewer::ImageViewer;

#[component]
pub fn PostCover(src: String) -> Element {
    rsx! {
        figure { class: "entry-cover",
            ImageViewer {
                src: src.clone(),
                thumb_params: "?w=1200",
                alt: "封面图片".to_string(),
                lazy_load: false,
            }
        }
    }
}

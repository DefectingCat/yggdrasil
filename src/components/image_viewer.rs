use dioxus::prelude::*;

#[component]
pub fn ImageViewer(
    src: String,
    #[props(default = "?w=800".to_string())]
    thumb_params: String,
    #[props(default = "图片".to_string())]
    alt: String,
    #[props(default = false)]
    lazy_load: bool,
) -> Element {
    let mut is_open = use_signal(|| false);

    let thumb_src = if src.contains('?') {
        format!("{}&{}", src.split('?').next().unwrap_or(&src), thumb_params.trim_start_matches('?'))
    } else {
        format!("{}{}", src, thumb_params)
    };

    rsx! {
        // Thumbnail
        img {
            class: "cursor-pointer transition-opacity hover:opacity-90",
            src: "{thumb_src}",
            alt: "{alt}",
            loading: if lazy_load { "lazy" } else { "eager" },
            onclick: move |_| is_open.set(true),
        }

        // Full-screen lightbox
        if is_open() {
            div {
                class: "image-viewer-overlay",
                onclick: move |_| is_open.set(false),
                div {
                    class: "image-viewer-content",
                    onclick: move |evt: dioxus::events::MouseEvent| evt.stop_propagation(),
                    img {
                        class: "image-viewer-img",
                        src: "{src}",
                        alt: "{alt}",
                    }
                    button {
                        class: "image-viewer-close",
                        onclick: move |_| is_open.set(false),
                        "✕"
                    }
                }
            }
        }
    }
}

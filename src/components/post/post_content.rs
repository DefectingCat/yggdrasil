use dioxus::prelude::*;

#[component]
pub fn PostContent(content_html: String) -> Element {
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        let _ = js_sys::eval(include_str!("../../../public/js/post-content.js"));
        let _ = js_sys::eval("window.__initPostContent('.post-content')");
    });

    rsx! {
        div {
            class: "post-content md-content",
            dangerous_inner_html: "{content_html}"
        }
    }
}

use dioxus::prelude::*;

#[component]
pub fn PostContent(content_html: String) -> Element {
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                // Add copy buttons to all code blocks
                if let Ok(elements) = document.query_selector_all("pre > code") {
                    for i in 0..elements.length() {
                        if let Some(codeblock) = elements.item(i) {
                            if let Some(parent) = codeblock.parent_element() {
                                // Check if button already exists
                                if parent.query_selector(".copy-code").ok().flatten().is_some() {
                                    continue;
                                }
                                
                                let copybutton = document.create_element("button").unwrap();
                                copybutton.set_class_name("copy-code");
                                copybutton.set_text_content(Some("copy"));
                                copybutton.set_attribute("aria-label", "Copy code").unwrap();
                                
                                let _ = parent.append_child(&copybutton);
                            }
                        }
                    }
                }
            }
        }
    });

    rsx! {
        div {
            class: "post-content md-content",
            dangerous_inner_html: "{content_html}"
        }
    }
}

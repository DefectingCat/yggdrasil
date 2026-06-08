use dioxus::prelude::*;

#[component]
pub fn PostContent(content_html: String) -> Element {
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

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

                // Add click-to-zoom for images in post content
                if let Ok(images) = document.query_selector_all(".md-content img") {
                    for i in 0..images.length() {
                        if let Some(img) = images.item(i) {
                                let img_element = img.clone().dyn_into::<web_sys::HtmlImageElement>().unwrap();
                            
                            // Skip if already processed
                            if img_element.get_attribute("data-zoom-enabled").is_some() {
                                continue;
                            }
                            img_element.set_attribute("data-zoom-enabled", "true").unwrap();
                            
                            let document_clone = document.clone();
                            let original_src = img_element.src();
                            let alt = img_element.alt();
                            
                            // Replace src with thumbnail version (add ?w=800)
                            let thumb_src = if original_src.contains('?') {
                                format!("{}&w=800", original_src)
                            } else {
                                format!("{}?w=800", original_src)
                            };
                            img_element.set_src(&thumb_src);
                            img_element.set_class_name("md-content-img-zoomable");
                            
                            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                                // Create overlay
                                let overlay = document_clone.create_element("div").unwrap();
                                overlay.set_class_name("md-image-lightbox-overlay");
                                
                                // Create image container
                                let container = document_clone.create_element("div").unwrap();
                                container.set_class_name("md-image-lightbox-content");
                                
                                // Create full-size image
                                let full_img = document_clone.create_element("img").unwrap();
                                let full_img_el = full_img.dyn_ref::<web_sys::HtmlImageElement>().unwrap();
                                full_img_el.set_src(&original_src);
                                full_img_el.set_alt(&alt);
                                
                                // Create close button
                                let close_btn = document_clone.create_element("button").unwrap();
                                close_btn.set_class_name("md-image-lightbox-close");
                                close_btn.set_text_content(Some("✕"));
                                
                                // Assemble
                                let _ = container.append_child(&full_img);
                                let _ = container.append_child(&close_btn);
                                let _ = overlay.append_child(&container);
                                let _ = document_clone.body().unwrap().append_child(&overlay);
                                
                                // Prevent body scroll
                                document_clone.body().unwrap().set_attribute("style", "overflow: hidden;").unwrap();
                                
                                // Click overlay background to close
                                let overlay_for_bg = overlay.clone();
                                let close_bg = Closure::wrap(Box::new(move |_evt: web_sys::MouseEvent| {
                                    if let Some(parent) = overlay_for_bg.parent_node() {
                                        let _ = parent.remove_child(&overlay_for_bg);
                                    }
                                }) as Box<dyn FnMut(_)>);
                                overlay.add_event_listener_with_callback("click", close_bg.as_ref().unchecked_ref()).unwrap();
                                close_bg.forget();
                                
                                // Click container stops propagation (so clicking image doesn't close)
                                let stop_prop = Closure::wrap(Box::new(move |evt: web_sys::MouseEvent| {
                                    evt.stop_propagation();
                                }) as Box<dyn FnMut(_)>);
                                container.add_event_listener_with_callback("click", stop_prop.as_ref().unchecked_ref()).unwrap();
                                stop_prop.forget();
                                
                                // Click close button to close
                                let overlay_for_btn = overlay.clone();
                                let close_btn_handler = Closure::wrap(Box::new(move |_evt: web_sys::MouseEvent| {
                                    if let Some(parent) = overlay_for_btn.parent_node() {
                                        let _ = parent.remove_child(&overlay_for_btn);
                                    }
                                }) as Box<dyn FnMut(_)>);
                                close_btn.add_event_listener_with_callback("click", close_btn_handler.as_ref().unchecked_ref()).unwrap();
                                close_btn_handler.forget();
                                
                                // Escape key to close
                                let overlay_for_key = overlay.clone();
                                let key_handler = Closure::wrap(Box::new(move |evt: web_sys::KeyboardEvent| {
                                    if evt.key() == "Escape" {
                                        if let Some(parent) = overlay_for_key.parent_node() {
                                            let _ = parent.remove_child(&overlay_for_key);
                                        }
                                    }
                                }) as Box<dyn FnMut(_)>);
                                document_clone.add_event_listener_with_callback("keydown", key_handler.as_ref().unchecked_ref()).unwrap();
                                key_handler.forget();
                                
                            }) as Box<dyn FnMut(_)>);
                            
                            img.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
                            closure.forget();
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

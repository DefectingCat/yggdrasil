use dioxus::prelude::*;

#[component]
pub fn Footer() -> Element {
    let mut visible = use_signal(|| false);

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                let closure = wasm_bindgen::prelude::Closure::wrap(Box::new(move || {
                    if let Some(w) = web_sys::window() {
                        let threshold = w.inner_height().ok()
                            .and_then(|h| h.as_f64())
                            .unwrap_or(0.0);
                        let scroll_y = w.scroll_y().unwrap_or(0.0);
                        let new_visible = scroll_y > threshold;
                        visible.set(new_visible);
                    }
                }) as Box<dyn FnMut()>);

                let _ = window.add_event_listener_with_callback("scroll", wasm_bindgen::JsCast::unchecked_ref(closure.as_ref()));

                let threshold = window.inner_height().ok()
                    .and_then(|h| h.as_f64())
                    .unwrap_or(0.0);
                let scroll_y = window.scroll_y().unwrap_or(0.0);
                visible.set(scroll_y > threshold);

                closure.forget();
            }
        }
    });

    let link_class = use_memo(move || {
        let base = "p-2 rounded-full cursor-pointer hover:opacity-80 transition-all duration-300 text-gray-600 dark:text-gray-300";
        if visible() {
            format!("{} opacity-100 translate-y-0", base)
        } else {
            format!("{} opacity-0 translate-y-2 pointer-events-none", base)
        }
    });

    rsx! {
        footer { class: "w-full border-t border-gray-200 dark:border-[#333] mt-auto",
            div { class: "max-w-3xl mx-auto px-6 py-5 flex items-center justify-between text-sm text-gray-400 dark:text-[#9b9c9d]",
                span { "© 2026 Yggdrasil Blog" }
                a {
                    class: "{link_class}",
                    href: "#top",
                    aria_label: "go to top",
                    title: "Go to Top (Alt + G)",
                    accesskey: "g",
                    onclick: move |evt| {
                        evt.prevent_default();
                        scroll_to_top();
                    },
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        height: "24px",
                        view_box: "0 -960 960 960",
                        width: "24px",
                        fill: "currentColor",
                        path {
                            d: "m296-224-56-56 240-240 240 240-56 56-184-183-184 183Zm0-240-56-56 240-240 240 240-56 56-184-183-184 183Z",
                        }
                    }
                }
            }
        }
    }
}

fn scroll_to_top() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            let mut options = web_sys::ScrollToOptions::new();
            options.top(0.0);
            options.behavior(web_sys::ScrollBehavior::Smooth);
            let _ = window.scroll_to_with_scroll_to_options(&options);

            if let Ok(history) = window.history() {
                let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(" "));
            }
        }
    }
}

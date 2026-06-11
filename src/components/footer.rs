use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[component]
#[allow(unused_mut)]
pub fn Footer() -> Element {
    let mut visible = use_signal(|| false);

    #[cfg(target_arch = "wasm32")]
    let listener_state = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(wasm_bindgen::prelude::Closure<dyn FnMut()>, web_sys::Window)>,
        ))
    });

    #[cfg(not(target_arch = "wasm32"))]
    let _listener_state = use_hook(|| Rc::new(RefCell::new(None::<()>)));

    #[cfg(target_arch = "wasm32")]
    let listener_state_for_effect = listener_state.clone();

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                let closure = wasm_bindgen::prelude::Closure::wrap(Box::new(move || {
                    if let Some(w) = web_sys::window() {
                        let threshold = w
                            .inner_height()
                            .ok()
                            .and_then(|h| h.as_f64())
                            .unwrap_or(0.0);
                        let scroll_y = w.scroll_y().unwrap_or(0.0);
                        let new_visible = scroll_y > threshold;
                        visible.set(new_visible);
                    }
                })
                    as Box<dyn FnMut()>);

                let _ = window.add_event_listener_with_callback(
                    "scroll",
                    wasm_bindgen::JsCast::unchecked_ref(closure.as_ref()),
                );

                let threshold = window
                    .inner_height()
                    .ok()
                    .and_then(|h| h.as_f64())
                    .unwrap_or(0.0);
                let scroll_y = window.scroll_y().unwrap_or(0.0);
                visible.set(scroll_y > threshold);

                *listener_state_for_effect.borrow_mut() = Some((closure, window));
            }
        }
    });

    #[cfg(target_arch = "wasm32")]
    use_drop(move || {
        if let Some((closure, window)) = listener_state.borrow_mut().take() {
            let _ = window.remove_event_listener_with_callback(
                "scroll",
                wasm_bindgen::JsCast::unchecked_ref(closure.as_ref()),
            );
        }
    });

    let btn_class = use_memo(move || {
        let base = "fixed bottom-16 right-8 z-50 w-10 h-10 rounded-full bg-paper-entry border border-paper-border shadow-sm flex items-center justify-center cursor-pointer transition-all duration-300 text-paper-secondary hover:text-paper-accent";
        if visible() {
            format!("{} opacity-100 translate-y-0", base)
        } else {
            format!("{} opacity-0 translate-y-2 pointer-events-none", base)
        }
    });

    rsx! {
        footer { class: "w-full border-t border-paper-border mt-auto",
            div { class: "max-w-3xl mx-auto px-6 py-5 flex items-center justify-between text-sm text-paper-secondary",
                span { "© 2026 Yggdrasil" }
            }
        }
        a {
            class: "{btn_class}",
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

fn scroll_to_top() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            let options = web_sys::ScrollToOptions::new();
            options.set_top(0.0);
            options.set_behavior(web_sys::ScrollBehavior::Smooth);
            let _ = window.scroll_to_with_scroll_to_options(&options);

            if let Ok(history) = window.history() {
                let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(" "));
            }
        }
    }
}

use dioxus::prelude::*;

pub const INPUT_CLASS: &str = "w-full px-4 py-2 border border-paper-border rounded-lg bg-paper-entry text-paper-primary placeholder:text-paper-tertiary focus:outline-none focus:border-paper-accent focus:ring-1 focus:ring-paper-accent/30 transition-colors duration-200";

pub const BUTTON_PRIMARY_CLASS: &str = "w-full py-2.5 px-4 bg-paper-accent text-white font-medium rounded-full hover:brightness-110 active:scale-[0.98] transition-all duration-200 cursor-pointer";

#[component]
pub fn FormInput(
    r#type: &'static str,
    placeholder: &'static str,
    value: String,
    oninput: EventHandler<String>,
    onkeydown: Option<EventHandler<KeyboardEvent>>,
) -> Element {
    rsx! {
        input {
            class: "{INPUT_CLASS}",
            r#type: "{r#type}",
            placeholder: "{placeholder}",
            value: "{value}",
            oninput: move |e| oninput.call(e.value()),
            onkeydown: move |e| {
                if let Some(ref handler) = onkeydown {
                    handler.call(e);
                }
            },
        }
    }
}

#[component]
pub fn FormLabel(label: &'static str) -> Element {
    rsx! {
        label { class: "block text-sm font-medium text-paper-secondary mb-1",
            "{label}"
        }
    }
}

#[component]
pub fn AlertBox(message: String, variant: &'static str) -> Element {
    let (bg_class, text_class) = match variant {
        "error" => ("bg-red-100 dark:bg-red-900/30", "text-red-700 dark:text-red-300"),
        "success" => ("bg-green-100 dark:bg-green-900/30", "text-green-700 dark:text-green-300"),
        _ => ("bg-paper-code-bg", "text-paper-secondary"),
    };
    rsx! {
        div { class: "mb-4 p-3 {bg_class} {text_class} rounded-lg text-center",
            "{message}"
        }
    }
}

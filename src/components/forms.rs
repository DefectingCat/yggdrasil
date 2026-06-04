use dioxus::prelude::*;

pub const INPUT_CLASS: &str = "w-full px-4 py-2 border border-gray-200 dark:border-[#333] rounded-lg bg-white dark:bg-[#2e2e33] text-gray-900 dark:text-[#dadadb] focus:outline-none focus:border-gray-400 dark:focus:border-gray-600";

pub const BUTTON_PRIMARY_CLASS: &str = "w-full py-2 px-4 bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 font-medium rounded-full hover:opacity-80 transition-opacity cursor-pointer";

pub const BUTTON_SECONDARY_CLASS: &str = "px-6 py-2 bg-gray-200 dark:bg-[#333] text-gray-700 dark:text-[#dadadb] rounded-full font-medium hover:opacity-80 transition-opacity cursor-pointer";

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
        label { class: "block text-sm font-medium text-gray-700 dark:text-[#9b9c9d] mb-1",
            "{label}"
        }
    }
}

#[component]
pub fn AlertBox(message: String, variant: &'static str) -> Element {
    let (bg_class, text_class) = match variant {
        "error" => ("bg-red-100 dark:bg-red-900/30", "text-red-700 dark:text-red-300"),
        "success" => ("bg-green-100 dark:bg-green-900/30", "text-green-700 dark:text-green-300"),
        _ => ("bg-gray-100 dark:bg-[#333]", "text-gray-700 dark:text-[#9b9c9d]"),
    };
    rsx! {
        div { class: "mb-4 p-3 {bg_class} {text_class} rounded-lg text-center",
            "{message}"
        }
    }
}

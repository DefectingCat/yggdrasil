use dioxus::prelude::*;

#[allow(dead_code)]
const THEME_KEY: &str = "yggdrasil-theme";

#[derive(Clone, Copy, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }
}

pub fn use_theme() -> Signal<Theme> {
    let theme = use_signal(|| {
        #[cfg(target_arch = "wasm32")]
        {
            let storage = web_sys::window()
                .and_then(|w| w.local_storage().ok())
                .flatten();
            if let Some(storage) = storage {
                if let Ok(Some(value)) = storage.get_item(THEME_KEY) {
                    if value == "dark" {
                        return Theme::Dark;
                    }
                }
            }
        }
        Theme::Light
    });

    use_effect(move || {
        let current = theme();
        let theme_str = current.as_str();

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(html) = document.document_element() {
                        let _ = html.set_attribute("data-theme", theme_str);
                    }
                }
                if let Some(storage) = window.local_storage().ok().flatten() {
                    let _ = storage.set_item(THEME_KEY, theme_str);
                }
            }
        }

        let _ = theme_str;
    });

    theme
}

#[component]
pub fn ThemeToggle() -> Element {
    let mut theme = use_theme();

    rsx! {
        button {
            class: "theme-toggle p-2 rounded-full bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors",
            onclick: move |_| theme.set(theme().toggle()),
            if theme() == Theme::Dark {
                "🌙"
            } else {
                "☀️"
            }
        }
    }
}

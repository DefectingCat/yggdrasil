use dioxus::prelude::*;

#[allow(dead_code)]
const THEME_KEY: &str = "yggdrasil-theme";

#[derive(Clone, Copy, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn toggle(&self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }
}

fn detect_initial_theme() -> Theme {
    #[cfg(target_arch = "wasm32")]
    {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return Theme::Light,
        };

        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(value)) = storage.get_item(THEME_KEY) {
                return if value == "dark" {
                    Theme::Dark
                } else {
                    Theme::Light
                };
            }
        }

        if let Ok(Some(media)) = window.match_media("(prefers-color-scheme: dark)") {
            if media.matches() {
                return Theme::Dark;
            }
        }
    }

    #[cfg(feature = "server")]
    {
        if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
            let parts = ctx.parts_mut();
            if let Some(cookie) = parts.headers.get("cookie") {
                if let Ok(cookie_str) = cookie.to_str() {
                    if cookie_str.contains("theme=dark") {
                        return Theme::Dark;
                    }
                }
            }
        }
    }

    Theme::Light
}

pub fn use_theme_provider() -> Signal<Theme> {
    let theme = use_signal(detect_initial_theme);

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let current = theme();
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(html) = document.document_element() {
                        match current {
                            Theme::Dark => {
                                let _ = html.class_list().add_1("dark");
                            }
                            Theme::Light => {
                                let _ = html.class_list().remove_1("dark");
                            }
                        }
                    }
                }
                if let Ok(Some(storage)) = window.local_storage() {
                    let theme_str = match current {
                        Theme::Dark => "dark",
                        Theme::Light => "light",
                    };
                    let _ = storage.set_item(THEME_KEY, theme_str);
                }
            }
        }
    });

    use_context_provider(|| theme);
    theme
}

pub fn use_theme() -> Signal<Theme> {
    use_context::<Signal<Theme>>()
}

const THEME_PRELOAD_SCRIPT: &str = r#"
(function() {
    try {
        var theme = localStorage.getItem('yggdrasil-theme');
        if (theme === 'dark' || (!theme && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
            document.documentElement.classList.add('dark');
        }
    } catch (e) {}
})();
"#;

#[component]
pub fn ThemePreload() -> Element {
    rsx! {
        script {
            dangerous_inner_html: "{THEME_PRELOAD_SCRIPT}",
        }
    }
}

#[component]
pub fn ThemeToggle() -> Element {
    let mut theme = use_theme();

    rsx! {
        button {
            class: "theme-toggle p-2 rounded-full cursor-pointer hover:opacity-80 transition-opacity text-gray-600 dark:text-gray-300",
            onclick: move |_| theme.set(theme().toggle()),
            if theme() == Theme::Dark {
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    height: "24px",
                    view_box: "0 -960 960 960",
                    width: "24px",
                    fill: "currentColor",
                    path {
                        d: "M484-80q-84 0-157.5-32t-128-86.5Q144-253 112-326.5T80-484q0-146 93-257.5T410-880q-18 99 11 193.5T521-521q71 71 165.5 100T880-410q-26 144-138 237T484-80Zm0-80q88 0 163-44t118-121q-86-8-163-43.5T464-465q-61-61-97-138t-43-163q-77 43-120.5 118.5T160-484q0 135 94.5 229.5T484-160Zm-20-305Z",
                    }
                }
            } else {
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    height: "24px",
                    view_box: "0 -960 960 960",
                    width: "24px",
                    fill: "currentColor",
                    path {
                        d: "M440-800v-120h80v120h-80Zm0 760v-120h80v120h-80Zm360-400v-80h120v80H800Zm-760 0v-80h120v80H40Zm708-252-56-56 70-72 58 58-72 70ZM198-140l-58-58 72-70 56 56-70 72Zm564 0-70-72 56-56 72 70-58 58ZM212-692l-72-70 58-58 70 72-56 56Zm98 382q-70-70-70-170t70-170q70-70 170-70t170 70q70 70 70 170t-70 170q-70 70-170 70t-170-70Zm283.5-56.5Q640-413 640-480t-46.5-113.5Q547-640 480-640t-113.5 46.5Q320-547 320-480t46.5 113.5Q413-320 480-320t113.5-46.5ZM480-480Z",
                    }
                }
            }
        }
    }
}

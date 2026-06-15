//! 主题（浅色 / 深色）管理。
//!
//! 提供两条初始化路径：
//! - **SSR**：从 HTTP 请求 Cookie 中的 `theme` 字段检测主题，避免首屏闪烁。
//! - **WASM 客户端**：优先读取 `localStorage` 中的持久化主题；不存在时回退到
//!   `prefers-color-scheme` 媒体查询；切换时同步更新 DOM class 与 localStorage。

use dioxus::prelude::*;

/// localStorage 中存储主题值的键名。
#[allow(dead_code)]
const THEME_KEY: &str = "yggdrasil-theme";

/// 应用主题枚举。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    /// 浅色主题。
    Light,
    /// 深色主题。
    Dark,
}

impl Theme {
    /// 切换到相反主题。
    pub fn toggle(&self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }
}

/// 检测初始主题。
///
/// 在 WASM 客户端优先读取 localStorage，回退到系统颜色偏好；
/// 在 SSR 阶段解析请求 Cookie；否则默认浅色主题。
fn detect_initial_theme() -> Theme {
    #[cfg(target_arch = "wasm32")]
    {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return Theme::Light,
        };

        // 优先读取 localStorage 中持久化的主题值。
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(value)) = storage.get_item(THEME_KEY) {
                return if value == "dark" {
                    Theme::Dark
                } else {
                    Theme::Light
                };
            }
        }

        // 没有持久化值时，根据系统颜色偏好决定。
        if let Ok(Some(media)) = window.match_media("(prefers-color-scheme: dark)") {
            if media.matches() {
                return Theme::Dark;
            }
        }
    }

    #[cfg(feature = "server")]
    {
        // SSR 路径：从请求 Cookie 中解析 `theme` 字段。
        if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
            if let Some(cookie) = ctx.parts_mut().headers.get("cookie") {
                if let Ok(cookie_str) = cookie.to_str() {
                    // 按 ';' 分割 Cookie 字符串，再按 '=' 分割键值对。
                    for cookie_pair in cookie_str.split(';') {
                        let mut parts = cookie_pair.trim().splitn(2, '=');
                        if let (Some(name), Some(value)) = (parts.next(), parts.next()) {
                            if name == "theme" && value == "dark" {
                                return Theme::Dark;
                            }
                        }
                    }
                }
            }
        }
    }

    Theme::Light
}

/// 提供主题上下文的 Hook。
///
/// 初始化时按 SSR Cookie → WASM localStorage → 系统偏好的顺序检测主题；
/// 主题变化时同步更新 HTML 根元素的 `dark` class 与 localStorage。
pub fn use_theme_provider() -> Signal<Theme> {
    let theme = use_signal(detect_initial_theme);

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let current = theme();
            if let Some(window) = web_sys::window() {
                // 同步 HTML 根元素的 dark class，用于 Tailwind dark mode。
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
                // 将当前主题持久化到 localStorage。
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

/// 读取当前主题 Signal 的 Hook。
///
/// 需在 `use_theme_provider` 之后的组件树中使用。
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

/// 首屏主题预加载脚本组件。
///
/// 通过内联脚本在页面渲染前读取 localStorage / 系统偏好并设置 `dark` class，
/// 防止主题切换时出现闪烁。
#[component]
pub fn ThemePreload() -> Element {
    rsx! {
        script {
            dangerous_inner_html: "{THEME_PRELOAD_SCRIPT}",
        }
    }
}

/// 主题切换按钮组件。
#[component]
pub fn ThemeToggle() -> Element {
    let mut theme = use_theme();
    let mut mounted = use_signal(|| false);

    use_effect(move || {
        mounted.set(true);
    });

    rsx! {
        button {
            class: "theme-toggle p-2 rounded-full cursor-pointer hover:text-paper-accent transition-colors duration-200 text-paper-secondary",
            onclick: move |_| theme.set(theme().toggle()),
            if mounted() && theme() == Theme::Dark {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_switches_light_to_dark() {
        assert_eq!(Theme::Light.toggle(), Theme::Dark);
    }

    #[test]
    fn toggle_switches_dark_to_light() {
        assert_eq!(Theme::Dark.toggle(), Theme::Light);
    }

    #[test]
    fn toggle_is_an_involution() {
        // 连续切换两次应当回到原始主题。
        assert_eq!(Theme::Light.toggle().toggle(), Theme::Light);
        assert_eq!(Theme::Dark.toggle().toggle(), Theme::Dark);
    }

    #[test]
    fn theme_derives_equality() {
        // Theme 派生了 PartialEq，相同变体必须相等。
        assert_eq!(Theme::Light, Theme::Light);
        assert_eq!(Theme::Dark, Theme::Dark);
        assert_ne!(Theme::Light, Theme::Dark);
    }

    #[test]
    fn theme_preload_script_adds_dark_class() {
        // 预加载脚本必须包含给 documentElement 添加 dark class 的逻辑。
        assert!(THEME_PRELOAD_SCRIPT.contains("classList.add('dark')"));
    }

    #[test]
    fn theme_preload_script_reads_local_storage() {
        // 预加载脚本必须读取 yggdrasil-theme 键，与 THEME_KEY 保持一致。
        assert!(THEME_PRELOAD_SCRIPT.contains("localStorage.getItem('yggdrasil-theme')"));
        assert_eq!(THEME_KEY, "yggdrasil-theme");
    }

    #[test]
    fn theme_preload_script_falls_back_to_prefers_color_scheme() {
        // 当 localStorage 中无主题时，脚本应回退到系统颜色偏好。
        assert!(THEME_PRELOAD_SCRIPT.contains("prefers-color-scheme: dark"));
    }

    #[test]
    fn theme_preload_script_swallows_errors() {
        // 预加载脚本必须包裹在 try/catch 中，避免禁用 localStorage 时抛错。
        assert!(THEME_PRELOAD_SCRIPT.contains("try"));
        assert!(THEME_PRELOAD_SCRIPT.contains("catch"));
    }
}

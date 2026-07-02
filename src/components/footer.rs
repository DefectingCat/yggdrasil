//! 页脚组件
//!
//! 提供站点版权信息，并在用户向下滚动超过一屏后显示"回到顶部"悬浮按钮。
//! 回到顶部的滚动监听与平滑滚动逻辑仅在 WASM 前端生效。

#[cfg(target_arch = "wasm32")]
use crate::hooks::event_listener::use_event_listener;
use dioxus::prelude::*;

/// 页脚与回到顶部按钮组件。
///
/// Props：无。
/// 关键行为：
/// - 监听窗口滚动，超过一屏时显示回到顶部按钮
/// - 点击按钮平滑滚动到顶部，并清理 URL 中的 `#`
/// - 滚动监听与平滑滚动仅在 `target_arch = "wasm32"` 下执行
#[component]
#[allow(unused_mut)]
pub fn Footer() -> Element {
    let mut visible = use_signal(|| false);

    // 根据 window 当前滚动位置同步 visible（注册监听后立即调用一次，避免首屏漏判）。
    // 滚动事件回调里也复用同一份判断逻辑。
    let mut sync_visible = move || {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(w) = web_sys::window() {
                let threshold = w
                    .inner_height()
                    .ok()
                    .and_then(|h| h.as_f64())
                    .unwrap_or(0.0);
                let scroll_y = w.scroll_y().unwrap_or(0.0);
                visible.set(scroll_y > threshold);
            }
        }
    };

    // 注册 scroll 监听：注册 / 卸载清理由 use_event_listener 负责。
    // 仅 WASM 端调用（server 端 use_event_listener 是 noop，但 acquire 闭包内的
    // web_sys 在非 wasm 下不可解析，故整块 cfg；hook 数量在 server build 中不影响，
    // 因为 server 端该组件只跑一次 SSR）。
    #[cfg(target_arch = "wasm32")]
    use_event_listener(
        || web_sys::window(),
        "scroll",
        // 滚动事件触发时复用同样的阈值判断。
        move || sync_visible(),
    );

    // 挂载时根据当前滚动位置初始化一次按钮可见性。
    use_effect(move || {
        sync_visible();
    });

    // 根据 visible 动态切换按钮显示/隐藏样式
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
                path { d: "m296-224-56-56 240-240 240 240-56 56-184-183-184 183Zm0-240-56-56 240-240 240 240-56 56-184-183-184 183Z" }
            }
        }
    }
}

/// 平滑滚动到页面顶部，并清理 history 中的 `#` 哈希。
///
/// 仅在 `target_arch = "wasm32"` 下执行实际滚动，SSR 环境中为空操作。
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

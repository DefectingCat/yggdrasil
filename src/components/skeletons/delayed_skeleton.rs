use dioxus::prelude::*;

/// 骨架屏显示前的最小延迟（毫秒）
/// 加载时间低于此值时骨架屏不会显示，避免闪烁
const SKELETON_DELAY_MS: u32 = 200;

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: u32) {
    use wasm_bindgen::JsCast;
    let js_code = format!("new Promise(r => setTimeout(r, {}))", ms);
    if let Ok(promise_val) = js_sys::eval(&js_code) {
        if let Ok(promise) = promise_val.dyn_into::<js_sys::Promise>() {
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn sleep_ms(ms: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
}

/// 延迟显示的骨架屏包装组件
///
/// 骨架屏区域始终占位，但初始时不可见（opacity-0）。
/// 延迟 SKELETON_DELAY_MS 毫秒后，如果仍在加载，则淡入显示。
/// 如果加载很快（< 200ms），用户完全看不到骨架屏。
#[component]
pub fn DelayedSkeleton(children: Element) -> Element {
    let mut visible = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            sleep_ms(SKELETON_DELAY_MS).await;
            visible.set(true);
        });
    });

    rsx! {
        div {
            class: if visible() {
                "opacity-100 transition-opacity duration-150"
            } else {
                "opacity-0"
            },
            {children}
        }
    }
}

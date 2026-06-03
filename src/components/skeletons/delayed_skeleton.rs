use dioxus::prelude::*;

/// 骨架屏 pulse 动画延迟（毫秒）
/// 加载时间低于此值时骨架屏只显示静态灰色块，避免 pulse 动画一闪而过
const SKELETON_PULSE_DELAY_MS: u32 = 200;

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

/// 延迟 pulse 动画的骨架屏包装组件
///
/// 骨架屏区域**立即显示**（灰色静态占位块），避免空白闪烁。
/// 延迟 SKELETON_PULSE_DELAY_MS 毫秒后，如果仍在加载，则启动 pulse 动画。
///
/// 快加载（< 200ms）：用户只看到静态灰色块一闪而过，无动画感知
/// 慢加载：灰色块正常 pulse，提示正在加载
#[component]
pub fn DelayedSkeleton(children: Element) -> Element {
    let mut pulsing = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            sleep_ms(SKELETON_PULSE_DELAY_MS).await;
            pulsing.set(true);
        });
    });

    rsx! {
        div {
            class: if pulsing() { "animate-pulse" } else { "" },
            {children}
        }
    }
}

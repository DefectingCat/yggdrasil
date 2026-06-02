use dioxus::prelude::*;

/// 骨架屏最小显示延迟（毫秒）。加载时间低于此值时不会显示骨架屏，避免闪烁。
pub const MIN_SKELETON_DELAY_MS: u32 = 200;

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

/// 延迟加载状态 Hook。
///
/// 当 `is_loading` 返回 true 时，延迟 `MIN_SKELETON_DELAY_MS` 毫秒后才返回 true；
/// 当 `is_loading` 返回 false 时，立即返回 false。
///
/// 用于骨架屏：避免数据加载很快时出现骨架屏一闪而过的问题。
pub fn use_delayed_loading<F>(is_loading: F) -> Signal<bool>
where
    F: Fn() -> bool + Clone + 'static,
{
    let mut should_show = use_signal(|| false);

    use_effect(move || {
        let loading = is_loading();

        if loading {
            if !should_show() {
                let is_loading_clone = is_loading.clone();
                spawn(async move {
                    sleep_ms(MIN_SKELETON_DELAY_MS).await;
                    if is_loading_clone() {
                        should_show.set(true);
                    }
                });
            }
        } else {
            should_show.set(false);
        }
    });

    should_show
}

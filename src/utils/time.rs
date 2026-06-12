//! 跨平台时间/睡眠工具。
//!
//! 根据目标架构分别实现：
//! - `wasm32`：通过 `js_sys` 调用 JavaScript 的 `setTimeout`。
//! - 其他平台：使用 `tokio::time::sleep`。

/// 异步睡眠指定毫秒数。
#[cfg(target_arch = "wasm32")]
pub async fn sleep_ms(ms: u32) {
    use wasm_bindgen::JsCast;
    let js_code = format!("new Promise(r => setTimeout(r, {}))", ms);
    if let Ok(promise_val) = js_sys::eval(&js_code) {
        if let Ok(promise) = promise_val.dyn_into::<js_sys::Promise>() {
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        }
    }
}

/// 异步睡眠指定毫秒数（原生 tokio 版本）。
#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep_ms(ms: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
}

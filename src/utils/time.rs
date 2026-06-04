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

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep_ms(ms: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
}

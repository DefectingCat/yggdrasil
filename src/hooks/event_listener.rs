//! 通用 DOM 事件监听 hook：封装 add/remove_event_listener 生命周期。
//!
//! 用 `use_hook` 持有 `(Closure, target)`，`use_effect` 注册，`use_drop` 清理，
//! 把 theme/footer 两处手写的样板收口到一处。非 wasm32 目标下整体编译为空操作
//! （SSR 无 DOM），调用方无需再写占位变量或 `#[cfg]`。
//!
//! 设计要点：
//! - `target` 通过一个初始化闭包 `acquire` 提供，在 `use_effect` 首次运行时调用
//!   （此时 DOM 一定可用）；这样调用方无需自己再包一层 `use_effect`。
//! - `event` 要求 `&'static str`，add 与 remove 复用同一字符串字面量。
//! - handler 是 `FnMut()`（无参）；若调用方需要 target 信息（如 `media.matches()`），
//!   应在 `acquire` 里 clone 一份后 move 进 handler。

/// 注册一次性事件监听器，组件卸载时自动移除。
///
/// - `acquire`：在 `use_effect` 首次运行（DOM 已就绪）时调用，返回要监听的 target
///   及其克隆（handler 内若要读 target，用这个克隆）。返回 `None` 则不注册。
/// - `event`：事件名（如 `"scroll"` / `"change"`），需为 `&'static str`。
/// - `handler`：事件触发时的回调，`FnMut()`（无参）。
///
/// 仅在 wasm32 执行真实注册；非 wasm 为 noop。
///
/// # 例
///
/// ```ignore
/// use_event_listener(
///     || web_sys::window().unwrap(),
///     "scroll",
///     move || { visible.set(/* ... */); },
/// );
/// ```
#[cfg(target_arch = "wasm32")]
pub fn use_event_listener<T, A, F>(acquire: A, event: &'static str, mut handler: F)
where
    T: AsRef<web_sys::EventTarget> + Clone + 'static,
    A: FnOnce() -> Option<T>,
    F: FnMut() + 'static,
{
    use std::cell::RefCell;
    use std::rc::Rc;

    // 用 use_hook 持有 (Closure, target)，在整个组件生命周期内复用；
    // use_drop 时 take 出来移除监听，防止泄漏。
    let state: Rc<RefCell<Option<(wasm_bindgen::prelude::Closure<dyn FnMut()>, T)>>> =
        use_hook(|| Rc::new(RefCell::new(None)));
    let state_for_drop = state.clone();

    use_effect(move || {
        let Some(target) = acquire() else { return };
        let target_clone = target.clone();
        let closure = wasm_bindgen::prelude::Closure::wrap(Box::new(move || {
            handler();
        })
            as Box<dyn FnMut()>);
        let _ = target_clone.as_ref().add_event_listener_with_callback(
            event,
            wasm_bindgen::JsCast::unchecked_ref(closure.as_ref()),
        );
        *state.borrow_mut() = Some((closure, target_clone));
    });

    use_drop(move || {
        if let Some((closure, tgt)) = state_for_drop.borrow_mut().take() {
            let _ = tgt.as_ref().remove_event_listener_with_callback(
                event,
                wasm_bindgen::JsCast::unchecked_ref(closure.as_ref()),
            );
        }
    });
}

/// 非 wasm 占位：SSR 无 DOM，编译为空操作。
///
/// 保留与 wasm 版相同的泛型形态，调用方代码两端完全一致。
#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_variables, dead_code)]
pub fn use_event_listener<T, A, F>(acquire: A, event: &'static str, handler: F)
where
    T: Clone + 'static,
    A: FnOnce() -> Option<T>,
    F: FnMut() + 'static,
{
    // SSR 无 DOM，空实现。
}

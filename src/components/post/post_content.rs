//! 文章内容组件
//!
//! 渲染由服务端生成的文章 HTML 内容，并在 WASM 前端初始化交互脚本。

use dioxus::prelude::*;

/// 读取 `window` 上的可选全局函数并调用;函数未定义/为 null 时静默跳过。
///
/// 替代 `js_sys::eval("if(window.__x) window.__x(...)")` 字符串拼贴模式:用
/// `Reflect::get` 取属性 + `Function::apply` 调用,无字符串求值,与 `tiptap_bridge`
/// 的类型化 extern 风格一致。
#[cfg(target_arch = "wasm32")]
fn invoke_optional_global(window: &web_sys::Window, name: &str, args: &[wasm_bindgen::JsValue]) {
    use wasm_bindgen::JsCast;
    if let Ok(fn_val) = js_sys::Reflect::get(window, &name.into()) {
        if !fn_val.is_undefined() && !fn_val.is_null() {
            let arr = js_sys::Array::new();
            for a in args {
                arr.push(a);
            }
            let _ = fn_val
                .unchecked_into::<js_sys::Function>()
                .apply(window, &arr);
        }
    }
}

/// 文章内容组件。
///
/// Props：
/// - `content_html`：服务端渲染的文章 HTML 字符串
///
/// 关键行为：
/// - 在 `target_arch = "wasm32"` 环境下调用 `window.__initPostContent` 初始化代码块
///   复制按钮（`yggdrasil-core.js` 已由 `Dioxus.toml` 全局注入）。
///   灯箱（图片灯箱 + 懒加载）改由 `Dioxus.toml` 全局注入 `lightbox.js`，
///   这里仅设置其初始化配置 `__lightboxSelectors` 并兜底调用。
#[component]
pub fn PostContent(content_html: String) -> Element {
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        let window = web_sys::window().unwrap();

        // 调用 window.__initPostContent('.post-content')：函数不存在时静默跳过
        // (与旧 eval 中的 if 守卫语义一致)。
        invoke_optional_global(&window, "__initPostContent", &[".post-content".into()]);

        // lightbox 改由 Dioxus.toml 全局 <script src> 加载（不再 include_str!）。
        // 双保险契约：先设配置,若 lightbox.js 已加载则立即调用;
        // 否则 lightbox.js 加载完后其 IIFE 尾部读到配置自启动。
        let selectors = js_sys::Array::new();
        selectors.push(&".post-content".into());
        selectors.push(&".entry-cover".into());
        let selectors_val = js_sys::Object::from(selectors).into();
        let _ = js_sys::Reflect::set(&window, &"__lightboxSelectors".into(), &selectors_val);
        invoke_optional_global(&window, "__initLightbox", &[selectors_val]);
    });

    rsx! {
        div {
            class: "post-content md-content",
            dangerous_inner_html: "{content_html}",
        }
    }
}

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::components::write_skeleton::WriteSkeleton;

#[component]
pub fn WritePage() -> Element {
    let mut title = use_signal(|| "".to_string());
    let mut content = use_signal(|| "".to_string());
    let mut loading = use_signal(|| true);

    // 初始化 Tiptap 编辑器
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = js_sys::eval(
                r#"
                (function initEditor() {
                    // 如果已经初始化过，直接标记为就绪
                    if (window.__tiptap_ready) return;

                    var container = document.getElementById('tiptap-editor');
                    if (!container) {
                        setTimeout(initEditor, 50);
                        return;
                    }
                    if (typeof window.TiptapEditor !== 'undefined' && window.TiptapEditor) {
                        window.TiptapEditor.create('tiptap-editor', {
                            content: '',
                            placeholder: '在此输入内容...',
                            onUpdate: function(markdown) {
                                window.__tiptap_content = markdown;
                            }
                        });
                        window.__tiptap_ready = true;
                        return;
                    }
                    setTimeout(initEditor, 50);
                })();
                "#,
            );
        }
    });

    // 轮询编辑器就绪状态
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                loop {
                    if let Ok(promise_val) = js_sys::eval("new Promise(r => setTimeout(r, 100))") {
                        if let Ok(promise) = promise_val.dyn_into::<js_sys::Promise>() {
                            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                        }
                    }
                    if let Ok(ready) = js_sys::eval("window.__tiptap_ready") {
                        if ready.as_bool().unwrap_or(false) {
                            loading.set(false);
                            break;
                        }
                    }
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            loading.set(false);
        }
    });

    rsx! {
        div { class: "space-y-4 relative",
            // 骨架屏覆盖层：编辑器初始化期间显示
            if loading() {
                div { class: "absolute inset-0 z-10 bg-white dark:bg-[#1d1e20]",
                    WriteSkeleton {}
                }
            }

            // 真实内容始终渲染，确保 #tiptap-editor 在 DOM 中
            // 初始化期间被骨架屏遮住，就绪后骨架屏消失
            input {
                class: "w-full text-2xl font-bold bg-transparent border-b border-gray-200 dark:border-[#333] py-2 mb-4 text-gray-900 dark:text-[#dadadb] placeholder-gray-400 dark:placeholder-[#9b9c9d] focus:outline-none",
                placeholder: "文章标题...",
                value: "{title}",
                oninput: move |evt| title.set(evt.value()),
            }

            div {
                class: "w-full h-[600px] border border-gray-200 dark:border-[#333] rounded-lg overflow-hidden bg-white dark:bg-[#1e1e1e]",
                id: "tiptap-editor",
            }

            button {
                class: "mt-4 px-6 py-2 bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full font-medium hover:opacity-80 transition-opacity",
                onclick: move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        let md = js_sys::eval(r#"
                            (function() {
                                var editor = window.TiptapEditor && window.TiptapEditor._instances && window.TiptapEditor._instances.get('tiptap-editor');
                                return editor ? editor.getMarkdown() : (window.__tiptap_content || '');
                            })()
                        "#).ok().and_then(|v| v.as_string()).unwrap_or_default();
                        content.set(md.clone());
                        println!("保存文章: title={}, content_len={}", title(), md.len());
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        println!("保存文章: title={}, content_len={}", title(), content().len());
                    }
                },
                "保存草稿"
            }
        }
    }
}

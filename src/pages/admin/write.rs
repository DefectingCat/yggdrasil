use dioxus::prelude::*;

use crate::components::admin_layout::AdminLayout;

#[component]
pub fn WritePage() -> Element {
    let mut title = use_signal(|| "".to_string());
    let mut content = use_signal(|| "".to_string());

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = js_sys::eval(
                r#"
                (function initEditor() {
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
                        return;
                    }
                    setTimeout(initEditor, 50);
                })();
                "#,
            );
        }
    });

    rsx! {
        AdminLayout {
            div { class: "space-y-4",
                // 标题输入
                input {
                    class: "w-full text-2xl font-bold bg-transparent border-b border-gray-200 dark:border-[#333] py-2 mb-4 text-gray-900 dark:text-[#dadadb] placeholder-gray-400 dark:placeholder-[#9b9c9d] focus:outline-none",
                    placeholder: "文章标题...",
                    value: "{title}",
                    oninput: move |evt| title.set(evt.value()),
                }

                // Tiptap 编辑器容器
                div {
                    class: "w-full h-[600px] border border-gray-200 dark:border-[#333] rounded-lg overflow-hidden bg-white dark:bg-[#1e1e1e]",
                    id: "tiptap-editor",
                }

                // 保存按钮
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
}

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::api::posts::{create_post, CreatePostResponse};
use crate::components::write_skeleton::WriteSkeleton;

#[component]
#[allow(unused_mut, unused_variables)]
pub fn Write() -> Element {
    let mut title = use_signal(|| "".to_string());
    let mut summary = use_signal(|| "".to_string());
    let mut slug = use_signal(|| "".to_string());
    let mut tags = use_signal(|| "".to_string());
    let mut status = use_signal(|| "draft".to_string());
    let mut content = use_signal(|| "".to_string());
    let mut loading = use_signal(|| true);
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);

    // 初始化 Tiptap 编辑器
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = js_sys::eval(
                r#"
                (function initEditor() {
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

    let on_submit = move |_| {
        if title().trim().is_empty() {
            error.set(Some("标题不能为空".to_string()));
            return;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let md = js_sys::eval(r#"
                (function() {
                    var editor = window.TiptapEditor && window.TiptapEditor._instances && window.TiptapEditor._instances.get('tiptap-editor');
                    return editor ? editor.getMarkdown() : (window.__tiptap_content || '');
                })()
            "#).ok().and_then(|v| v.as_string()).unwrap_or_default();

            if md.trim().is_empty() {
                error.set(Some("内容不能为空".to_string()));
                return;
            }

            let tags_list: Vec<String> = tags()
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();

            let slug_opt = if slug().trim().is_empty() {
                None
            } else {
                Some(slug().trim().to_string())
            };

            let summary_opt = if summary().trim().is_empty() {
                None
            } else {
                Some(summary().trim().to_string())
            };

            saving.set(true);
            error.set(None);

            spawn(async move {
                match create_post(title().trim().to_string(), slug_opt, summary_opt, md, status(), tags_list).await {
                    Ok(CreatePostResponse { success: true, .. }) => {
                        saving.set(false);
                        success.set(true);
                        // Delay navigation slightly so user sees success message
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = js_sys::eval("new Promise(r => setTimeout(r, 800))");
                        }
                        let _ = dioxus::router::navigator().push("/admin");
                    }
                    Ok(CreatePostResponse { success: false, message, .. }) => {
                        saving.set(false);
                        error.set(Some(message));
                    }
                    Err(e) => {
                        saving.set(false);
                        error.set(Some(format!("保存失败: {}", e)));
                    }
                }
            });
        }
    };

    rsx! {
        div { class: "space-y-4 relative",
            // 骨架屏覆盖层：编辑器初始化期间显示
            if loading() {
                div { class: "absolute inset-0 z-10 bg-white dark:bg-[#1d1e20]",
                    WriteSkeleton {}
                }
            }

            // 标题
            input {
                class: "w-full text-2xl font-bold bg-transparent border-b border-gray-200 dark:border-[#333] py-2 mb-2 text-gray-900 dark:text-[#dadadb] placeholder-gray-400 dark:placeholder-[#9b9c9d] focus:outline-none",
                placeholder: "文章标题...",
                value: "{title}",
                oninput: move |evt| title.set(evt.value()),
            }

            // 摘要
            textarea {
                class: "w-full text-sm bg-transparent border-b border-gray-200 dark:border-[#333] py-2 mb-2 text-gray-700 dark:text-[#9b9c9d] placeholder-gray-400 dark:placeholder-[#9b9c9d] focus:outline-none resize-none",
                placeholder: "文章摘要（留空自动生成）",
                rows: "2",
                value: "{summary}",
                oninput: move |evt| summary.set(evt.value()),
            }

            // Slug + Tags + Status 行
            div { class: "flex flex-col md:flex-row gap-3 mb-2",
                input {
                    class: "flex-1 text-sm bg-transparent border-b border-gray-200 dark:border-[#333] py-2 text-gray-700 dark:text-[#9b9c9d] placeholder-gray-400 dark:placeholder-[#9b9c9d] focus:outline-none",
                    placeholder: "URL 标识（留空自动生成）",
                    value: "{slug}",
                    oninput: move |evt| slug.set(evt.value()),
                }
                input {
                    class: "flex-1 text-sm bg-transparent border-b border-gray-200 dark:border-[#333] py-2 text-gray-700 dark:text-[#9b9c9d] placeholder-gray-400 dark:placeholder-[#9b9c9d] focus:outline-none",
                    placeholder: "标签，用逗号分隔",
                    value: "{tags}",
                    oninput: move |evt| tags.set(evt.value()),
                }
                select {
                    class: "text-sm bg-transparent border-b border-gray-200 dark:border-[#333] py-2 text-gray-700 dark:text-[#9b9c9d] focus:outline-none cursor-pointer",
                    value: "{status}",
                    onchange: move |evt| status.set(evt.value()),
                    option { value: "draft", "草稿" }
                    option { value: "published", "发布" }
                }
            }

            // Tiptap 编辑器
            div {
                class: "w-full h-[500px] border border-gray-200 dark:border-[#333] rounded-lg overflow-hidden bg-white dark:bg-[#1e1e1e]",
                id: "tiptap-editor",
            }

            // 错误提示
            if let Some(err) = error() {
                div { class: "mt-4 p-3 bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 rounded-lg text-sm",
                    "{err}"
                }
            }

            // 成功提示
            if success() {
                div { class: "mt-4 p-3 bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded-lg text-sm",
                    "保存成功！"
                }
            }

            // 保存按钮
            div { class: "flex gap-3 mt-4",
                button {
                    class: if saving() {
                        "px-6 py-2 bg-gray-400 text-white rounded-full font-medium cursor-not-allowed"
                    } else {
                        "px-6 py-2 bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full font-medium hover:opacity-80 transition-opacity cursor-pointer"
                    },
                    disabled: saving(),
                    onclick: on_submit,
                    if saving() {
                        "保存中..."
                    } else {
                        "保存"
                    }
                }
                button {
                    class: "px-6 py-2 bg-gray-200 dark:bg-[#333] text-gray-700 dark:text-[#dadadb] rounded-full font-medium hover:opacity-80 transition-opacity cursor-pointer",
                    onclick: move |_| {
                        let _ = dioxus::router::navigator().push("/admin");
                    },
                    "取消"
                }
            }
        }
    }
}

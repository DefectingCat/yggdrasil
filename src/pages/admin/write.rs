use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::api::posts::{create_post, CreatePostResponse};
use crate::components::write_skeleton::WriteSkeleton;
use crate::router::Route;

#[component]
#[allow(unused_mut, unused_variables)]
pub fn Write() -> Element {
    let mut title = use_signal(|| "".to_string());
    let mut summary = use_signal(|| "".to_string());
    let mut slug = use_signal(|| "".to_string());
    let mut tags = use_signal(|| "".to_string());
    let mut cover_image = use_signal(|| "".to_string());
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
                        try {
                            window.TiptapEditor.create('tiptap-editor', {
                                content: '',
                                placeholder: '在此输入内容...',
                                onUpdate: function(markdown) {
                                    window.__tiptap_content = markdown;
                                },
                                onImageUpload: function(file) {
                                    return new Promise(function(resolve, reject) {
                                        var formData = new FormData();
                                        formData.append('image', file);

                                        fetch('/api/upload', {
                                            method: 'POST',
                                            body: formData,
                                            credentials: 'same-origin'
                                        })
                                        .then(function(response) {
                                            if (!response.ok) {
                                                throw new Error('Upload failed: ' + response.status);
                                            }
                                            return response.json();
                                        })
                                        .then(function(data) {
                                            if (data.success && data.url) {
                                                resolve(data.url);
                                            } else {
                                                reject(new Error(data.error || 'Upload failed'));
                                            }
                                        })
                                        .catch(function(err) {
                                            reject(err);
                                        });
                                    });
                                }
                            });
                            window.__tiptap_ready = true;
                        } catch(e) {
                            console.error('[tiptap] create error: ' + e.message);
                        }
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
                for i in 0..100 {
                    if let Ok(promise_val) = js_sys::eval("new Promise(r => setTimeout(r, 100))") {
                        if let Ok(promise) = promise_val.dyn_into::<js_sys::Promise>() {
                            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                        }
                    }
                    if let Ok(ready) = js_sys::eval("window.__tiptap_ready") {
                        if ready.as_bool().unwrap_or(false) {
                            loading.set(false);
                            return;
                        }
                    }
                }
                loading.set(false);
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

            let cover_image_opt = if cover_image().trim().is_empty() {
                None
            } else {
                Some(cover_image().trim().to_string())
            };

            saving.set(true);
            error.set(None);

            spawn(async move {
                match create_post(
                    title().trim().to_string(),
                    slug_opt,
                    summary_opt,
                    md,
                    status(),
                    tags_list,
                    cover_image_opt,
                )
                .await
                {
                    Ok(CreatePostResponse { success: true, .. }) => {
                        saving.set(false);
                        success.set(true);
                        // Delay navigation slightly so user sees success message
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = js_sys::eval("new Promise(r => setTimeout(r, 800))");
                        }
                        let _ = dioxus::router::navigator().push(Route::Admin {});
                    }
                    Ok(CreatePostResponse {
                        success: false,
                        message,
                        ..
                    }) => {
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
        div { class: "relative",
            if loading() {
                div { class: "absolute inset-0 z-10 bg-white dark:bg-[#1d1e20]",
                    WriteSkeleton {}
                }
            }

            div { class: "space-y-6",
                div { class: "rounded-xl bg-white dark:bg-[#2e2e33] border border-gray-200 dark:border-[#333] p-6 space-y-5",
                    input {
                        class: "w-full text-2xl font-bold bg-transparent text-gray-900 dark:text-[#dadadb] placeholder-gray-300 dark:placeholder-[#555] focus:outline-none",
                        placeholder: "文章标题",
                        value: "{title}",
                        oninput: move |evt| title.set(evt.value()),
                    }

                    textarea {
                        class: "w-full text-sm bg-gray-50 dark:bg-[#1d1e20] rounded-lg px-4 py-3 text-gray-700 dark:text-[#9b9c9d] placeholder-gray-400 dark:placeholder-[#555] focus:outline-none resize-none border border-gray-100 dark:border-[#333]",
                        placeholder: "摘要（留空则自动生成）",
                        rows: "2",
                        value: "{summary}",
                        oninput: move |evt| summary.set(evt.value()),
                    }

                    div { class: "grid grid-cols-1 md:grid-cols-3 gap-3",
                        div {
                            label { class: "block text-xs text-gray-600 dark:text-[#9b9c9d] mb-1.5 font-medium", "Slug" }
                            input {
                                class: "w-full text-sm bg-gray-50 dark:bg-[#1d1e20] rounded-lg px-3 py-2.5 text-gray-700 dark:text-[#9b9c9d] placeholder-gray-400 dark:placeholder-[#555] focus:outline-none border border-gray-100 dark:border-[#333]",
                                placeholder: "自动生成",
                                value: "{slug}",
                                oninput: move |evt| slug.set(evt.value()),
                            }
                        }
                        div {
                            label { class: "block text-xs text-gray-600 dark:text-[#9b9c9d] mb-1.5 font-medium", "标签" }
                            input {
                                class: "w-full text-sm bg-gray-50 dark:bg-[#1d1e20] rounded-lg px-3 py-2.5 text-gray-700 dark:text-[#9b9c9d] placeholder-gray-400 dark:placeholder-[#555] focus:outline-none border border-gray-100 dark:border-[#333]",
                                placeholder: "逗号分隔",
                                value: "{tags}",
                                oninput: move |evt| tags.set(evt.value()),
                            }
                        }
                        div {
                            label { class: "block text-xs text-gray-600 dark:text-[#9b9c9d] mb-1.5 font-medium", "封面图" }
                            input {
                                class: "w-full text-sm bg-gray-50 dark:bg-[#1d1e20] rounded-lg px-3 py-2.5 text-gray-700 dark:text-[#9b9c9d] placeholder-gray-400 dark:placeholder-[#555] focus:outline-none border border-gray-100 dark:border-[#333]",
                                placeholder: "URL（可选）",
                                value: "{cover_image}",
                                oninput: move |evt| cover_image.set(evt.value()),
                            }
                        }
                    }
                }

                div {
                    class: "w-full h-[500px] border border-gray-200 dark:border-[#333] rounded-lg overflow-hidden bg-white dark:bg-[#1e1e1e]",
                    id: "tiptap-editor",
                }

                if let Some(err) = error() {
                    div { class: "px-4 py-3 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded-xl text-sm border border-red-100 dark:border-red-900/30",
                        "{err}"
                    }
                }

                if success() {
                    div { class: "px-4 py-3 bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 rounded-xl text-sm border border-green-100 dark:border-green-900/30",
                        "保存成功"
                    }
                }

                div { class: "flex items-center gap-3 pt-2",
                    div { class: "flex-1" }
                    button {
                        class: "px-5 py-2.5 text-sm bg-gray-200 dark:bg-[#333] text-gray-700 dark:text-[#dadadb] rounded-full font-medium hover:opacity-80 transition-opacity cursor-pointer",
                        onclick: move |_| {
                            let _ = dioxus::router::navigator().push(Route::Admin {});
                        },
                        "取消"
                    }
                    div {
                        class: "relative inline-flex items-center px-5 py-2.5 text-sm bg-gray-50 dark:bg-[#1d1e20] border border-gray-200 dark:border-[#333] rounded-full text-gray-700 dark:text-[#9b9c9d] cursor-pointer min-w-[80px]",
                        select {
                            class: "absolute inset-0 w-full h-full opacity-0 cursor-pointer",
                            style: "appearance: none; -webkit-appearance: none;",
                            value: "{status}",
                            onchange: move |evt| status.set(evt.value()),
                            option { value: "draft", "草稿" }
                            option { value: "published", "发布" }
                        }
                        span { class: "pr-2",
                            if status() == "draft" { "草稿" } else { "发布" }
                        }
                        svg {
                            class: "h-4 w-4 text-gray-500 dark:text-[#666] pointer-events-none",
                            xmlns: "http://www.w3.org/2000/svg",
                            view_box: "0 0 20 20",
                            fill: "currentColor",
                            path {
                                fill_rule: "evenodd",
                                d: "M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z",
                                clip_rule: "evenodd"
                            }
                        }
                    }
                    button {
                        class: if saving() {
                            "px-6 py-2.5 text-sm bg-gray-400 text-white rounded-full font-medium cursor-not-allowed"
                        } else {
                            "px-6 py-2.5 text-sm bg-gray-900 dark:bg-[#dadadb] text-white dark:text-gray-900 rounded-full font-medium hover:opacity-80 transition-opacity cursor-pointer"
                        },
                        disabled: saving(),
                        onclick: on_submit,
                        if saving() { "保存中..." } else { "保存" }
                    }
                }
            }
        }
    }
}

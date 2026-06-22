//! 文章编辑器页面。
//!
//! 提供新建文章与编辑文章两种模式，使用基于 Tiptap 的富文本编辑器。
//! 编辑器通过 `js_sys::eval` 在 WASM 前端初始化，并与 `window.TiptapEditor` 实例交互，
//! 实现 Markdown 内容回填、图片上传与组件卸载时的清理。

use dioxus::prelude::*;

// 仅在 WASM 前端使用的类型转换与文章 API。
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
use crate::api::posts::{
    create_post, get_post_by_id, update_post, CreatePostResponse, SinglePostResponse,
};
use crate::components::write_skeleton::WriteSkeleton;
use crate::models::post::Post;
use crate::router::Route;

/// 当前编辑器内进行中的上传计数（来自轮询 window.__tiptap_uploads.counts）。
#[derive(Clone, Copy, Default)]
struct UploadsInFlight {
    uploading: u32,
    error: u32,
}

/// 顶部堆叠的上传失败提示条目。
#[derive(Clone, PartialEq)]
struct UploadErrorEntry {
    id: String,
    file_name: String,
    message: String,
}

/// window.__tiptap_uploads 快照（轮询消费用）。
#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
struct UploadSnapshot {
    events: Vec<UploadSnapshotEvent>,
    counts: UploadSnapshotCounts,
}

#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
struct UploadSnapshotEvent {
    kind: String,
    #[serde(rename = "uploadId")]
    upload_id: String,
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(rename = "errorMsg")]
    error_msg: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize, Default, Clone, Copy)]
struct UploadSnapshotCounts {
    uploading: u32,
    error: u32,
}

/// 新建文章页面组件。
///
/// 内部委托给 `write_editor`，以 `None` 表示新建模式。
#[component]
#[allow(unused_mut, unused_variables)]
pub fn Write() -> Element {
    write_editor(None)
}

/// 编辑文章页面组件。
///
/// `id` 为要编辑的文章 ID，内部委托给 `write_editor` 加载现有数据。
#[component]
#[allow(unused_mut, unused_variables)]
pub fn WriteEdit(id: i32) -> Element {
    write_editor(Some(id))
}

/// 文章编辑器核心组件，支持新建（`post_id == None`）与编辑模式。
///
/// 负责：
/// - 编辑模式下通过 server function 拉取文章数据；
/// - 在 WASM 前端初始化 Tiptap 富文本编辑器并轮询就绪状态；
/// - 编辑模式下将 Markdown 内容回填到编辑器；
/// - 提交时读取编辑器 Markdown、校验并调用 create_post / update_post；
/// - 组件卸载时销毁 Tiptap 实例并清理全局状态。
#[allow(unused_mut, unused_variables)]
fn write_editor(post_id: Option<i32>) -> Element {
    let is_edit = post_id.is_some();

    // 文章元信息表单字段。
    let mut title = use_signal(|| "".to_string());
    let mut summary = use_signal(|| "".to_string());
    let mut slug = use_signal(|| "".to_string());
    let mut tags = use_signal(|| "".to_string());
    let mut cover_image = use_signal(|| "".to_string());
    let mut status = use_signal(|| "draft".to_string());
    let mut content = use_signal(|| "".to_string());
    // 页面与编辑器加载、保存、错误、成功等状态。
    let mut loading = use_signal(|| true);
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);
    let mut editor_content_set = use_signal(|| false);
    let mut has_backfilled = use_signal(|| false);
    let mut load_error = use_signal(|| None::<String>);

    // 编辑模式：用于暂存从服务端加载的文章数据。
    let mut edit_post = use_signal(|| None::<Post>);

    // 上传状态：当前进行中计数（保存拦截）+ 顶部失败提示堆叠（用户手动关闭）
    let mut uploads_in_flight = use_signal(UploadsInFlight::default);
    let mut upload_errors: Signal<Vec<UploadErrorEntry>> = use_signal(Vec::new);
    // 已展示过错误的上传 id（去重 + 重试后再失败时原地更新）。
    // 用 signal 而非局部变量：use_future 的 FnMut 闭包要求可重复调用，
    // signal 是 Copy 可被多次 move 进 async。
    let mut seen_error_ids: Signal<std::collections::HashSet<String>> =
        use_signal(std::collections::HashSet::new);

    // 编辑模式：文章数据加载完成后，将字段回填到表单信号。
    use_effect(move || {
        if !is_edit || has_backfilled() {
            return;
        }
        if let Some(ref post) = edit_post() {
            has_backfilled.set(true);
            title.set(post.title.clone());
            summary.set(post.summary.clone().unwrap_or_default());
            slug.set(post.slug.clone());
            tags.set(post.tags.join(", "));
            cover_image.set(post.cover_image.clone().unwrap_or_default());
            status.set(post.status.as_str().to_string());
            content.set(post.content_md.clone());
        }
    });

    // 编辑模式：仅在 WASM 前端通过 server function 加载文章详情。
    use_effect(move || {
        if is_edit {
            #[cfg(target_arch = "wasm32")]
            if let Some(id) = post_id {
                spawn(async move {
                    match get_post_by_id(id).await {
                        Ok(SinglePostResponse { post: Some(post) }) => {
                            edit_post.set(Some(post));
                        }
                        Ok(SinglePostResponse { post: None }) => {
                            load_error.set(Some("文章不存在".to_string()));
                        }
                        Err(e) => {
                            load_error.set(Some(format!("加载失败: {}", e)));
                        }
                    }
                });
            }
        }
    });

    // 组件卸载时清理 Tiptap 实例：销毁编辑器、删除实例映射、重置全局就绪标志与内容缓存。
    #[cfg(target_arch = "wasm32")]
    use_drop(move || {
        let _ = js_sys::eval(
            r#"
            (function() {
                var editor = window.TiptapEditor && window.TiptapEditor._instances && window.TiptapEditor._instances.get('tiptap-editor');
                if (editor && typeof editor.destroy === 'function') {
                    editor.destroy();
                }
                if (window.TiptapEditor && window.TiptapEditor._instances) {
                    window.TiptapEditor._instances.delete('tiptap-editor');
                }
                window.__tiptap_ready = false;
                window.__tiptap_content = '';
            })();
            "#,
        );
    });

    // Tiptap 编辑器初始化：在 WASM 前端通过 js_sys::eval 调用 public/tiptap/ 构建产物暴露的 window.TiptapEditor。
    // 编辑模式需等待文章数据加载完成，避免空内容覆盖后续回填。
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            // 编辑模式：等数据加载完再初始化。
            if is_edit && edit_post().is_none() {
                return;
            }

            // 执行 JS 初始化脚本：查找 DOM 容器，调用 TiptapEditor.create，并注册 onUpdate / onImageUpload 回调。
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
                                                // 读取服务端返回的中文错误（{"success":false,"error":"文件超过大小限制"}）
                                                return response.json().catch(function() { return null; }).then(function(data) {
                                                    if (data && data.error) {
                                                        throw new Error(data.error);
                                                    }
                                                    throw new Error('上传失败: ' + response.status);
                                                });
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

    // 轮询 Tiptap 编辑器就绪状态：最多等待约 10 秒，就绪后编辑模式回填 Markdown，最后结束加载。
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            // 编辑模式：等数据加载完再开始轮询。
            if is_edit && edit_post().is_none() {
                return;
            }

            wasm_bindgen_futures::spawn_local(async move {
                for i in 0..100 {
                    if let Ok(promise_val) = js_sys::eval("new Promise(r => setTimeout(r, 100))") {
                        if let Ok(promise) = promise_val.dyn_into::<js_sys::Promise>() {
                            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                        }
                    }
                    if let Ok(ready) = js_sys::eval("window.__tiptap_ready") {
                        if ready.as_bool().unwrap_or(false) {
                            // 编辑模式：通过 window.TiptapEditor 实例的 setMarkdown 回填已有内容。
                            if is_edit && !editor_content_set() {
                                let md = content();
                                if !md.is_empty() {
                                    let md_json = serde_json::to_string(&md).unwrap_or_default();
                                    let script = format!(
                                        "(function() {{ var editor = window.TiptapEditor && window.TiptapEditor._instances && window.TiptapEditor._instances.get('tiptap-editor'); if (editor) {{ editor.setMarkdown({}); }} }})()",
                                        md_json
                                    );
                                    let _ = js_sys::eval(&script);
                                }
                                editor_content_set.set(true);
                            }
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

    // 轮询 window.__tiptap_uploads，消费上传事件并更新 signal（仅 WASM）
    #[cfg(target_arch = "wasm32")]
    {
        use_future(move || {
            let mut uploads_in_flight = uploads_in_flight;
            let mut upload_errors = upload_errors;
            let mut seen_error_ids = seen_error_ids;
            async move {
                loop {
                    // 500ms 间隔
                    if let Ok(promise_val) = js_sys::eval("new Promise(r => setTimeout(r, 500))") {
                        if let Ok(promise) = promise_val.dyn_into::<js_sys::Promise>() {
                            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                        }
                    }

                    // 读取并清空 events，读取 counts
                    let snapshot = js_sys::eval(
                        r#"
                        (function() {
                            var u = window.__tiptap_uploads;
                            if (!u) return null;
                            var events = u.events || [];
                            u.events = [];
                            return JSON.stringify({ events: events, counts: u.counts || {uploading:0,error:0} });
                        })()
                        "#,
                    )
                    .ok()
                    .and_then(|v| v.as_string());

                    if let Some(json) = snapshot {
                        if let Ok(parsed) = serde_json::from_str::<UploadSnapshot>(&json) {
                            for ev in parsed.events {
                                match ev.kind.as_str() {
                                    "error" => {
                                        let msg = ev.error_msg.unwrap_or_else(|| "上传失败".to_string());
                                        if seen_error_ids.write().insert(ev.upload_id.clone()) {
                                            // 新失败：追加提示
                                            upload_errors.write().push(UploadErrorEntry {
                                                id: ev.upload_id,
                                                file_name: ev.file_name,
                                                message: msg,
                                            });
                                        } else {
                                            // 已存在的 id（重试后再失败）：原地更新消息，
                                            // 避免顶部提示停留在旧错误文案。
                                            let mut errors = upload_errors.write();
                                            if let Some(entry) = errors.iter_mut().find(|e| e.id == ev.upload_id) {
                                                entry.message = msg;
                                            }
                                        }
                                    }
                                    "success" | "removed" => {
                                        seen_error_ids.write().remove(&ev.upload_id);
                                        upload_errors.write().retain(|e| e.id != ev.upload_id);
                                    }
                                    _ => {}
                                }
                            }
                            uploads_in_flight.set(UploadsInFlight {
                                uploading: parsed.counts.uploading,
                                error: parsed.counts.error,
                            });
                        }
                    }
                }
            }
        });
    }

    // 提交表单：校验标题与内容，读取 Tiptap 编辑器 Markdown，调用 create_post 或 update_post。
    let on_submit = move |_| {
        // 上传未完成/失败拦截：有占位符时阻止保存
        let in_flight = uploads_in_flight.read();
        if in_flight.uploading > 0 || in_flight.error > 0 {
            let msg = if in_flight.uploading > 0 {
                format!("有 {} 张图片正在上传，请等待完成后再保存", in_flight.uploading)
            } else {
                format!("有 {} 张图片上传失败，请移除或重试后再保存", in_flight.error)
            };
            error.set(Some(msg));
            return;
        }
        drop(in_flight);

        if title().trim().is_empty() {
            error.set(Some("标题不能为空".to_string()));
            #[allow(clippy::needless_return)]
            return;
        }

        // 仅在 WASM 前端读取编辑器内容并发起保存请求。
        #[cfg(target_arch = "wasm32")]
        {
            // 优先通过 window.TiptapEditor 实例读取 Markdown，否则退回到全局缓存。
            let md = js_sys::eval(r#"
                (function() {
                    var editor = window.TiptapEditor && window.TiptapEditor._instances && window.TiptapEditor._instances.get('tiptap-editor');
                    return editor ? editor.getMarkdown() : (window.__tiptap_content || '');
                })()
            "#).ok().and_then(|v| v.as_string()).unwrap_or_default();

            // 兜底：扫描残留的上传占位符标记（轮询窗口期漏判防护）
            // 检测 ![](blob:...) 形式的泄漏图片 src，而非裸 "blob:" 字符串，
            // 避免误伤合法讨论 blob URL 的代码块/正文。
            if md.contains("](blob:") || md.contains("data-upload-state") {
                error.set(Some("检测到未完成上传的图片，请处理后保存".to_string()));
                return;
            }

            if md.trim().is_empty() {
                error.set(Some("内容不能为空".to_string()));
                return;
            }

            // 将逗号分隔的标签字符串转换为列表。
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

            if let Some(id) = post_id {
                // 编辑模式：调用 update_post
                spawn(async move {
                    match update_post(
                        id,
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
                            let _ = dioxus::router::navigator().push(Route::Posts {});
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
                            error.set(Some(format!("更新失败: {}", e)));
                        }
                    }
                });
            } else {
                // 新建模式：调用 create_post
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
        }
    };

    let save_button_text = if saving() {
        "保存中..."
    } else if is_edit {
        "更新"
    } else {
        "保存"
    };

    rsx! {
        div { class: "relative flex flex-col flex-1 min-h-0 overflow-hidden",
            if loading() {
                div { class: "absolute inset-0 z-10 bg-white dark:bg-[#1d1e20]",
                    WriteSkeleton {}
                }
            }

            // 顶部元信息区域 - 固定高度，不滚动
            div { class: "flex-shrink-0 space-y-5 pt-8",
                // 标题区域 - 大字号无框输入
                div {
                    input {
                        class: "w-full text-3xl md:text-4xl font-bold bg-transparent text-[var(--color-paper-primary)] placeholder-[var(--color-paper-tertiary)] focus:outline-none tracking-tight leading-tight",
                        placeholder: "文章标题",
                        value: "{title}",
                        oninput: move |evt| title.set(evt.value()),
                    }
                }

                // 摘要
                textarea {
                    class: "w-full text-base bg-transparent text-[var(--color-paper-secondary)] placeholder-[var(--color-paper-tertiary)] focus:outline-none resize-none leading-relaxed",
                    placeholder: "摘要（留空则自动生成）",
                    rows: "2",
                    value: "{summary}",
                    oninput: move |evt| summary.set(evt.value()),
                }

                // 元数据行 - 紧凑精致
                div { class: "flex flex-wrap items-end gap-x-8 gap-y-4 text-sm",
                    div { class: "flex-1 min-w-[140px]",
                        label { class: "block text-[11px] font-medium text-[var(--color-paper-secondary)] tracking-wider mb-2",
                            "Slug"
                        }
                        input {
                            class: "w-full text-sm bg-transparent text-[var(--color-paper-primary)] placeholder-[var(--color-paper-tertiary)] focus:outline-none border-b border-[var(--color-paper-tertiary)] focus:border-[var(--color-paper-primary)] transition-colors pb-1.5",
                            placeholder: "自动生成",
                            value: "{slug}",
                            oninput: move |evt| slug.set(evt.value()),
                        }
                    }
                    div { class: "flex-1 min-w-[140px]",
                        label { class: "block text-[11px] font-medium text-[var(--color-paper-secondary)] tracking-wider mb-2",
                            "标签"
                        }
                        input {
                            class: "w-full text-sm bg-transparent text-[var(--color-paper-primary)] placeholder-[var(--color-paper-tertiary)] focus:outline-none border-b border-[var(--color-paper-tertiary)] focus:border-[var(--color-paper-primary)] transition-colors pb-1.5",
                            placeholder: "逗号分隔",
                            value: "{tags}",
                            oninput: move |evt| tags.set(evt.value()),
                        }
                    }
                    div { class: "flex-1 min-w-[140px]",
                        label { class: "block text-[11px] font-medium text-[var(--color-paper-secondary)] tracking-wider mb-2",
                            "封面图"
                        }
                        input {
                            class: "w-full text-sm bg-transparent text-[var(--color-paper-primary)] placeholder-[var(--color-paper-tertiary)] focus:outline-none border-b border-[var(--color-paper-tertiary)] focus:border-[var(--color-paper-primary)] transition-colors pb-1.5",
                            placeholder: "URL（可选）",
                            value: "{cover_image}",
                            oninput: move |evt| cover_image.set(evt.value()),
                        }
                    }
                }
            }

            // 编辑器区域 - 沾满剩余高度
            div { class: "flex-1 min-h-0 flex flex-col my-4",
                div {
                    class: "flex-1 min-h-0 w-full border border-[var(--color-paper-border)] rounded-xl overflow-hidden bg-[var(--color-paper-entry)] shadow-[0_2px_8px_rgba(0,0,0,0.04)] dark:shadow-none",
                    id: "tiptap-editor",
                }
            }

            // 错误和成功提示
            if let Some(err) = load_error() {
                div { class: "flex-shrink-0 px-4 py-2 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded-xl text-sm border border-red-100 dark:border-red-900/30 mb-2",
                    "{err}"
                }
            }

            if let Some(err) = error() {
                div { class: "flex-shrink-0 px-4 py-2 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded-xl text-sm border border-red-100 dark:border-red-900/30 mb-2",
                    "{err}"
                }
            }

            if success() {
                div { class: "flex-shrink-0 px-4 py-2 bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 rounded-xl text-sm border border-green-100 dark:border-green-900/30 mb-2",
                    "保存成功"
                }
            }

            // 上传失败提示：多条堆叠，×关闭同时删除编辑器内失败占位符（避免孤儿）
            for err in upload_errors().clone() {
                div { class: "flex-shrink-0 flex items-center justify-between gap-3 px-4 py-2 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded-xl text-sm border border-red-100 dark:border-red-900/30 mb-2",
                    span { "图片上传失败: {err.file_name} — {err.message}" }
                    button {
                        class: "shrink-0 text-red-400 hover:text-red-600 cursor-pointer text-lg leading-none",
                        aria_label: "关闭提示",
                        onclick: {
                            // 捕获 owned id，避免借用临时值
                            let id = err.id.clone();
                            let mut upload_errors = upload_errors;
                            move |_| {
                                // 关闭提示同时删除编辑器内失败占位符（避免孤儿）
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let _ = js_sys::eval(&format!(
                                        "(function(){{var e=window.TiptapEditor&&window.TiptapEditor._instances&&window.TiptapEditor._instances.get('tiptap-editor');if(e&&e.removeUploadByUploadId){{e.removeUploadByUploadId({:?});}}}})()",
                                        id
                                    ));
                                }
                                upload_errors.write().retain(|e| e.id != id);
                            }
                        },
                        "×"
                    }
                }
            }

            // 底部操作栏 - 在编辑器下方，左对齐
            div { class: "flex-shrink-0 flex items-center gap-2 pt-2 pb-4",
                button {
                    class: "px-4 py-1.5 text-sm text-[var(--color-paper-secondary)] hover:text-[var(--color-paper-primary)] transition-colors cursor-pointer",
                    onclick: move |_| {
                        let _ = dioxus::router::navigator().push(Route::Posts {});
                    },
                    "取消"
                }
                div { class: "w-px h-5 bg-[var(--color-paper-border)]" }
                div {
                    class: "relative inline-flex items-center px-3 py-1.5 text-sm text-[var(--color-paper-secondary)] cursor-pointer",
                    select {
                        class: "absolute inset-0 w-full h-full opacity-0 cursor-pointer",
                        style: "appearance: none; -webkit-appearance: none;",
                        value: "{status}",
                        onchange: move |evt| status.set(evt.value()),
                        option { value: "draft", "草稿" }
                        option { value: "published", "发布" }
                    }
                    span { class: "pr-1.5 text-[var(--color-paper-primary)] font-medium",
                        if status() == "draft" { "草稿" } else { "发布" }
                    }
                    svg {
                        class: "h-3.5 w-3.5 text-[var(--color-paper-tertiary)] pointer-events-none",
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
                div { class: "w-px h-5 bg-[var(--color-paper-border)]" }
                button {
                    class: if saving() {
                        "px-5 py-1.5 text-sm bg-[var(--color-paper-tertiary)] text-[var(--color-paper-secondary)] rounded-xl font-medium cursor-not-allowed"
                    } else {
                        "px-5 py-1.5 text-sm bg-[var(--color-paper-primary)] text-[var(--color-paper-theme)] rounded-xl font-medium hover:opacity-90 transition-opacity cursor-pointer"
                    },
                    disabled: saving(),
                    onclick: on_submit,
                    "{save_button_text}"
                }
            }
        }
    }
}

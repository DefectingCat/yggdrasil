//! Tiptap 编辑器的 wasm-bindgen 绑定层。
//!
//! 封装与 `window.TiptapEditor`（IIFE 暴露的全局对象）的全部交互，
//! 替代旧版 `js_sys::eval` 字符串拼贴 + window 全局变量通信。
//!
//! 全模块仅在 WASM 前端使用（`#[cfg(target_arch = "wasm32")]`），
//! 因为 server 构建里无 `window` 对象。

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// —— window.TiptapEditor 模块对象 ——
#[wasm_bindgen]
extern "C" {
    pub type TiptapEditorModule;

    /// 读取 `window.TiptapEditor`（IIFE 默认导出，顶层 var 即 window 属性）。
    #[wasm_bindgen(js_namespace = window, js_name = TiptapEditor)]
    pub fn get_module() -> TiptapEditorModule;

    /// 调用 `TiptapEditor.create(containerId, options)`。
    /// 找不到容器返回 null（被 Option 捕获）；构造失败抛异常（被 catch 捕获）。
    #[wasm_bindgen(method, catch)]
    pub fn create(
        this: &TiptapEditorModule,
        container_id: &str,
        opts: &EditorOptions,
    ) -> Result<Option<EditorInstance>, JsValue>;
}

// —— 编辑器实例（TiptapEditorInstance）——
#[wasm_bindgen]
extern "C" {
    pub type EditorInstance;

    /// 富文本模式下返回 ProseMirror 的 Markdown；源码模式下返回 textarea 内容。
    #[wasm_bindgen(method, js_name = getMarkdown)]
    pub fn get_markdown(this: &EditorInstance) -> String;

    /// 用 Markdown 内容回填编辑器（emitUpdate: false，不触发 onUpdate）。
    #[wasm_bindgen(method, js_name = setMarkdown)]
    pub fn set_markdown(this: &EditorInstance, content: &str);

    /// 按 uploadId 删除上传节点（revoke blob + 删 pending）。供宿主"×关闭"调用。
    #[wasm_bindgen(method, js_name = removeUploadByUploadId)]
    pub fn remove_upload_by_upload_id(this: &EditorInstance, upload_id: &str) -> bool;

    /// 销毁编辑器，释放 JS 侧资源。
    #[wasm_bindgen(method)]
    pub fn destroy(this: &EditorInstance);
}

// —— EditorOptions：用 builder 模式（setter）构造 JS 对象 ——
#[wasm_bindgen]
extern "C" {
    pub type EditorOptions;

    #[wasm_bindgen(constructor)]
    pub fn new() -> EditorOptions;

    #[wasm_bindgen(method, setter, js_name = placeholder)]
    pub fn set_placeholder(this: &EditorOptions, v: &str);

    #[wasm_bindgen(method, setter, js_name = onUpdate)]
    pub fn set_on_update(this: &EditorOptions, cb: &Closure<dyn Fn(String)>);

    /// JS 侧 onImageUpload: (file: File) => Promise<string>。
    /// Rust closure 返回 js_sys::Promise，由 future_to_promise 包装。
    #[wasm_bindgen(method, setter, js_name = onImageUpload)]
    pub fn set_on_image_upload(
        this: &EditorOptions,
        cb: &Closure<dyn Fn(web_sys::File) -> js_sys::Promise>,
    );

    /// 编辑器就绪回调（init 末尾同步触发一次）。
    #[wasm_bindgen(method, setter, js_name = onReady)]
    pub fn set_on_ready(this: &EditorOptions, cb: &Closure<dyn Fn()>);

    /// 上传事件回调（coordinator.emit 时触发，携带 counts）。
    #[wasm_bindgen(method, setter, js_name = onUploadEvent)]
    pub fn set_on_upload_event(this: &EditorOptions, cb: &Closure<dyn Fn(UploadEventJs)>);
}

// —— 上传事件（JS UploadEvent 的 Rust 映射）——
#[wasm_bindgen]
extern "C" {
    #[derive(Clone)]
    pub type UploadEventJs;

    #[wasm_bindgen(method, getter)]
    pub fn kind(this: &UploadEventJs) -> String;

    #[wasm_bindgen(method, getter, js_name = uploadId)]
    pub fn upload_id(this: &UploadEventJs) -> String;

    #[wasm_bindgen(method, getter, js_name = fileName)]
    pub fn file_name(this: &UploadEventJs) -> String;

    #[wasm_bindgen(method, getter, js_name = errorMsg)]
    pub fn error_msg(this: &UploadEventJs) -> Option<String>;

    #[wasm_bindgen(method, getter)]
    pub fn counts(this: &UploadEventJs) -> UploadCountsJs;
}

#[wasm_bindgen]
extern "C" {
    #[derive(Clone, Copy)]
    pub type UploadCountsJs;

    #[wasm_bindgen(method, getter)]
    pub fn uploading(this: &UploadCountsJs) -> u32;

    #[wasm_bindgen(method, getter)]
    pub fn error(this: &UploadCountsJs) -> u32;
}

// —— 共享类型（供 write.rs 与本模块内部使用）——

/// 当前编辑器内进行中的上传计数（来自 onUploadEvent 的 counts 快照）。
#[derive(Clone, Copy, Default)]
pub struct UploadsInFlight {
    pub uploading: u32,
    pub error: u32,
}

/// 顶部堆叠的上传失败提示条目。
#[derive(Clone, PartialEq)]
pub struct UploadErrorEntry {
    pub id: String,
    pub file_name: String,
    pub message: String,
}

// —— EditorHandle：实例 + closure 统一生命周期 ——

/// 持有编辑器实例 + 其全部 closure，统一生命周期。
///
/// drop 时先 destroy JS 实例（释放 ProseMirror 资源），再 drop closure
/// （释放 wasm-bindgen 回调表）。比 `Closure::forget`（永久泄漏）干净——
/// closure 严格随编辑器实例同生共死。
///
/// 字段顺序：`instance` 在前仅表示逻辑主从；Drop::drop 里显式调 destroy，
/// 之后 closure 字段按声明逆序自动 drop，无顺序敏感问题。
pub struct EditorHandle {
    instance: EditorInstance,
    _on_update: Closure<dyn Fn(String)>,
    _on_image_upload: Closure<dyn Fn(web_sys::File) -> js_sys::Promise>,
    _on_ready: Closure<dyn Fn()>,
    _on_upload_event: Closure<dyn Fn(UploadEventJs)>,
}

impl EditorHandle {
    pub fn new(
        instance: EditorInstance,
        on_update: Closure<dyn Fn(String)>,
        on_image_upload: Closure<dyn Fn(web_sys::File) -> js_sys::Promise>,
        on_ready: Closure<dyn Fn()>,
        on_upload_event: Closure<dyn Fn(UploadEventJs)>,
    ) -> Self {
        Self {
            instance,
            _on_update: on_update,
            _on_image_upload: on_image_upload,
            _on_ready: on_ready,
            _on_upload_event: on_upload_event,
        }
    }

    /// 访问底层编辑器实例（调 getMarkdown/setMarkdown/destroy 等）。
    pub fn instance(&self) -> &EditorInstance {
        &self.instance
    }
}

impl Drop for EditorHandle {
    fn drop(&mut self) {
        self.instance.destroy();
        // closure 字段随后自动 drop（逆序：on_upload_event → on_ready → on_image_upload → on_update）
    }
}

// —— consume_upload_event：纯逻辑 helper ——

/// 消费单个上传事件，更新 signal（即时驱动，替代旧版 500ms 轮询）。
///
/// 逻辑与旧轮询 body 一致：
/// - error：新 id 追加提示，已存在 id 原地更新消息（重试后再失败）
/// - success/removed：移除对应提示 + 清 seen
/// - counts：直接从事件读（JS 已遍历文档算好）
#[allow(clippy::too_many_arguments)]
pub fn consume_upload_event(
    ev: &UploadEventJs,
    uploads_in_flight: &mut dioxus::prelude::Signal<UploadsInFlight>,
    upload_errors: &mut dioxus::prelude::Signal<Vec<UploadErrorEntry>>,
    seen_error_ids: &mut dioxus::prelude::Signal<std::collections::HashSet<String>>,
) {
    let id = ev.upload_id();
    match ev.kind().as_str() {
        "error" => {
            let msg = ev.error_msg().unwrap_or_else(|| "上传失败".to_string());
            if seen_error_ids.write().insert(id.clone()) {
                // 新失败：追加提示
                upload_errors.write().push(UploadErrorEntry {
                    id: id.clone(),
                    file_name: ev.file_name(),
                    message: msg,
                });
            } else {
                // 已存在的 id（重试后再失败）：原地更新消息
                let mut errors = upload_errors.write();
                if let Some(entry) = errors.iter_mut().find(|e| e.id == id) {
                    entry.message = msg;
                }
            }
        }
        "success" | "removed" => {
            seen_error_ids.write().remove(&id);
            upload_errors.write().retain(|e| e.id != id);
        }
        _ => {}
    }
    // counts 直接从事件读（JS 已算好）
    let c = ev.counts();
    uploads_in_flight.set(UploadsInFlight {
        uploading: c.uploading(),
        error: c.error(),
    });
}

// —— make_upload_closure：Rust fetch 上传 ——

/// 创建图片上传 closure：FormData POST /api/upload，解析 {success, url, error}。
///
/// 行为与旧版 write.rs eval 注入的 JS fetch 等价：
/// - credentials: same-origin（携带 session cookie）
/// - 字段名 'image'（与服务端 upload.rs 对齐）
/// - 成功 → resolve(url)；失败 → reject(Error(服务端中文 error))
///
/// 返回的 closure 签名 `(File) -> Promise` 对应 JS `onImageUpload`。
pub fn make_upload_closure() -> Closure<dyn Fn(web_sys::File) -> js_sys::Promise> {
    Closure::new(move |file: web_sys::File| -> js_sys::Promise {
        // 构造 FormData：字段名 'image' 与服务端 upload.rs 对齐
        let form =
            web_sys::FormData::new().expect("FormData::new failed");
        form.append_with_blob("image", &file)
            .expect("FormData.append failed");

        // 构造 POST 请求，credentials same-origin 携带 session cookie
        let mut init = web_sys::RequestInit::new();
        init.method("POST");
        init.body(Some(&form));
        init.credentials(web_sys::RequestCredentials::SameOrigin);

        let request = web_sys::Request::new_with_str_and_init("/api/upload", &init)
            .expect("Request::new failed");

        let window = web_sys::window().expect("no window");
        let promise = window.fetch_with_request(&request);

        // 把 fetch Promise → Future → 解析响应体 → 再包回 Promise
        wasm_bindgen_futures::future_to_promise(async move {
            let resp_val = wasm_bindgen_futures::JsFuture::from(promise).await?;
            let resp: web_sys::Response = resp_val.dyn_into()?;

            // 读响应体文本（无论 2xx 与否，服务端都返回 JSON）
            let text_promise = resp
                .text()
                .map_err(|e| js_sys::Error::new(&format!("读取响应失败: {:?}", e)))?;
            let text_val = wasm_bindgen_futures::JsFuture::from(text_promise).await?;
            let text = text_val.as_string().unwrap_or_default();

            // 解析 {success, url, error}
            let data: serde_json::Value =
                serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);

            if data["success"].as_bool() == Some(true) {
                let url = data["url"].as_str().unwrap_or("").to_string();
                Ok(js_sys::JsString::from(url).into())
            } else {
                // 失败：优先用服务端中文 error，兜底用状态码
                let msg = data["error"]
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("上传失败: {}", resp.status()));
                Err(js_sys::Error::new(&msg).into())
            }
        })
    })
}

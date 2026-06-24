//! Tiptap 编辑器的 wasm-bindgen 绑定层。
//!
//! 封装与 `window.TiptapEditor`（IIFE 暴露的全局对象）的全部交互，
//! 替代旧版 `js_sys::eval` 字符串拼贴 + window 全局变量通信。
//!
//! wasm-bindgen extern 与 `EditorHandle`、上传 closure 等**仅在 WASM 前端**编译
//! （server 构建无 `window`）；共享的纯数据类型 `UploadsInFlight`/`UploadErrorEntry`
//! 在两端都编译，供 `write.rs` 在 rsx 中渲染上传状态。

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

// ============================================================================
// 以下全部仅在 WASM 前端编译：wasm-bindgen extern + EditorHandle + 上传 closure。
// 放在 #[cfg] 子模块内，避免 server 构建尝试编译引用 JS 对象的 extern。
// ============================================================================
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use super::{UploadErrorEntry, UploadsInFlight};
    // WritableExt 提供 .write()（Signal 在 Copy 语义下不需要 mut 绑定）。
    use dioxus::prelude::WritableExt;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    // —— window.TiptapEditor 模块对象 ——
    //
    // TiptapEditor 是 IIFE 产物挂在 window 上的模块对象（含 create 等方法），
    // 不是函数。wasm-bindgen 对 `fn get_module() -> T` 形式的 extern 会生成
    // `window.TiptapEditor()`（函数调用），会因"not a function"失败。
    // 因此用 js_sys::Reflect::get 做属性访问拿到模块对象，再 dyn_into。
    #[wasm_bindgen]
    extern "C" {
        pub type TiptapEditorModule;

        /// 调用 `TiptapEditor.create(containerId, options)`。
        /// 找不到容器返回 null（被 Option 捕获）；构造失败抛异常（被 catch 捕获）。
        #[wasm_bindgen(method, catch)]
        pub fn create(
            this: &TiptapEditorModule,
            container_id: &str,
            opts: &EditorOptions,
        ) -> Result<Option<EditorInstance>, JsValue>;
    }

    /// 读取 `window.TiptapEditor`（IIFE 默认导出，顶层 var 即 window 属性）。
    /// 用 Reflect::get 做属性访问——extern fn 形式会被 wasm-bindgen 编成函数调用。
    pub fn get_module() -> TiptapEditorModule {
        let window = web_sys::window().expect("no window");
        let val = js_sys::Reflect::get(&window, &"TiptapEditor".into()).expect("window.TiptapEditor missing");
        val.dyn_into::<TiptapEditorModule>()
            .expect("window.TiptapEditor is not TiptapEditorModule")
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
    pub fn set_on_update(this: &EditorOptions, cb: &Closure<dyn FnMut(String)>);

    /// JS 侧 onImageUpload: (file: File) => Promise<string>。
    /// Rust closure 返回 js_sys::Promise，由 future_to_promise 包装。
    #[wasm_bindgen(method, setter, js_name = onImageUpload)]
    pub fn set_on_image_upload(
        this: &EditorOptions,
        cb: &Closure<dyn Fn(web_sys::File) -> js_sys::Promise>,
    );

    /// 编辑器就绪回调（init 末尾同步触发一次）。
    #[wasm_bindgen(method, setter, js_name = onReady)]
    pub fn set_on_ready(this: &EditorOptions, cb: &Closure<dyn FnMut()>);

    /// 上传事件回调（coordinator.emit 时触发，携带 counts）。
    #[wasm_bindgen(method, setter, js_name = onUploadEvent)]
    pub fn set_on_upload_event(this: &EditorOptions, cb: &Closure<dyn FnMut(UploadEventJs)>);
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
        #[derive(Clone)]
        pub type UploadCountsJs;

        #[wasm_bindgen(method, getter)]
        pub fn uploading(this: &UploadCountsJs) -> u32;

        #[wasm_bindgen(method, getter)]
        pub fn error(this: &UploadCountsJs) -> u32;
    }

    // —— EditorHandle：实例 + closure 统一生命周期 ——

    /// 持有编辑器实例 + 其全部 closure，统一生命周期。
    ///
    /// drop 时先 destroy JS 实例（释放 ProseMirror 资源），随后 closure 字段
    /// 按声明逆序自动 drop（释放 wasm-bindgen 回调表）。比 `Closure::forget`
    /// （永久泄漏）干净——closure 严格随编辑器实例同生共死。
    pub struct EditorHandle {
        instance: EditorInstance,
        _on_update: Closure<dyn FnMut(String)>,
        _on_image_upload: Closure<dyn Fn(web_sys::File) -> js_sys::Promise>,
        _on_ready: Closure<dyn FnMut()>,
        _on_upload_event: Closure<dyn FnMut(UploadEventJs)>,
    }

    impl EditorHandle {
        pub fn new(
            instance: EditorInstance,
            on_update: Closure<dyn FnMut(String)>,
            on_image_upload: Closure<dyn Fn(web_sys::File) -> js_sys::Promise>,
            on_ready: Closure<dyn FnMut()>,
            on_upload_event: Closure<dyn FnMut(UploadEventJs)>,
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
        }
    }

    // —— consume_upload_event：纯逻辑 helper ——

    /// 消费单个上传事件，更新 signal（即时驱动，替代旧版 500ms 轮询）。
    ///
    /// 逻辑与旧轮询 body 一致：
    /// - error：新 id 追加提示，已存在 id 原地更新消息（重试后再失败）
    /// - success/removed：移除对应提示
    /// - counts：直接从事件读（JS 已遍历文档算好）
    ///
    /// 去重以 `upload_errors` Vec 自身为唯一数据源（用 iter().any 判重），
    /// 不再额外维护 seen_error_ids，避免两份状态需手动同步。
    pub fn consume_upload_event(
        ev: &UploadEventJs,
        mut uploads_in_flight: dioxus::prelude::Signal<UploadsInFlight>,
        mut upload_errors: dioxus::prelude::Signal<Vec<UploadErrorEntry>>,
    ) {
        let id = ev.upload_id();
        match ev.kind().as_str() {
            "error" => {
                let msg = ev.error_msg().unwrap_or_else(|| "上传失败".to_string());
                // 已存在同 id（重试后再失败）：原地更新消息；否则追加。
                let mut errors = upload_errors.write();
                if let Some(entry) = errors.iter_mut().find(|e| e.id == id) {
                    entry.message = msg;
                } else {
                    errors.push(UploadErrorEntry {
                        id: id.clone(),
                        file_name: ev.file_name(),
                        message: msg,
                    });
                }
            }
            "success" | "removed" => {
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
    /// 构造阶段的错误（FormData/Request 等）以 rejected Promise 返回，
    /// 而非 panic —— 单张坏文件不应导致整个编辑器崩溃。
    pub fn make_upload_closure() -> Closure<dyn Fn(web_sys::File) -> js_sys::Promise> {
        Closure::new(move |file: web_sys::File| -> js_sys::Promise {
            wasm_bindgen_futures::future_to_promise(async move {
                // 构造 FormData：字段名 'image' 与服务端 upload.rs 对齐
                let form = web_sys::FormData::new()
                    .map_err(|_| js_sys::Error::new("无法构造上传表单"))?;
                form.append_with_blob("image", &file)
                    .map_err(|_| js_sys::Error::new("无法附加文件"))?;

                // 构造 POST 请求，credentials same-origin 携带 session cookie
                let init = web_sys::RequestInit::new();
                init.set_method("POST");
                // set_body 接收 &JsValue（非 Option）；FormData: AsRef<JsValue>。
                init.set_body(form.as_ref());
                init.set_credentials(web_sys::RequestCredentials::SameOrigin);

                let request = web_sys::Request::new_with_str_and_init("/api/upload", &init)
                    .map_err(|_| js_sys::Error::new("无法构造上传请求"))?;

                let window = web_sys::window().expect("no window");
                let promise = window.fetch_with_request(&request);

                // 把 fetch Promise → Future → 解析响应体 → 再包回 Promise
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
                    let url = data["url"].as_str().unwrap_or("");
                    if !url.is_empty() {
                        Ok(js_sys::JsString::from(url).into())
                    } else {
                        // success=true 但 url 为空：服务端契约异常，按失败处理
                        Err(js_sys::Error::new("上传成功但未返回图片地址").into())
                    }
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
}

#[cfg(target_arch = "wasm32")]
pub use wasm::{
    consume_upload_event, make_upload_closure, EditorHandle, EditorOptions, UploadEventJs,
    get_module,
};

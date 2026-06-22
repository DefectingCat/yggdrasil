# 编辑器图片上传占位符与失败提示设计

## 背景与目标

文章编辑器（`libs/tiptap-editor/`）已有图片上传功能（见 `2025-06-05-image-upload-design.md`），支持粘贴、拖拽、斜杠命令三种入口上传到 `/api/upload`。但当前上传过程存在两个体验缺陷：

1. **上传中无反馈**：图片仅在成功后才插入编辑器，用户在等待期间看不到任何"正在上传"的占位符，不知道发生了什么。
2. **上传失败完全静默**：三个上传入口（`FileHandler.onPaste`/`onDrop`、`SlashCommand`）失败时只有 `console.error`，用户在编辑器里什么都看不到。更糟的是，`write.rs` 的 fetch 还丢弃了服务端返回的中文错误（如"文件超过大小限制"），只拼成 `'Upload failed: 413'`。

本设计的目标：
- 上传过程中在编辑器内显示**占位符**（本地预览图 + loading 遮罩）
- 上传失败时把占位符变成**错误态卡片**（含服务端错误文案 + 重试/移除按钮），同时在页面顶部显示**可堆叠的失败提示**
- 保存文章时，若有未完成或失败的占位符，**阻止保存**

## 关键决策

| 决策点 | 选择 |
|--------|------|
| 占位符内容 | 本地预览图（blob URL）+ 半透明遮罩 + spinner + "上传中…" |
| 失败呈现 | 占位符变错误态（灰化 + 红色图标 + 服务端错误文案 + 重试/移除按钮）+ 顶部多条堆叠提示 |
| 重试机制 | 无限重试，File 对象保留在节点上直到成功或移除 |
| 顶部提示形态 | 静态多条堆叠，手动关闭（×） |
| 保存处理 | 有 loading/error 占位符时阻止保存，顶部提示 |
| 架构 | JS 主导（自定义 Image 节点视图）+ Rust 轮询全局变量获取上传状态 |

## 技术约束（探索结论）

- Tiptap Image 扩展（v3.25.0）**无原生上传状态**：`addAttributes` 只有 `src/alt/title/width/height`，没有 `uploading`/`uploadId` 之类。需要自定义扩展或 NodeView。
- 项目无 JS→Rust 事件通道：现有 eval 桥接是单向的（Rust eval 调 JS 方法读返回值，或 JS 写全局变量 Rust 轮询）。没有 wasm-bindgen 导出机制。失败提示要落到 Rust 的 Dioxus signal，必须新增一个轮询通道。
- 现有无 toast 组件：所有提示都是 signal 驱动的静态内联块（`AlertBox` 或 write.rs 自有的红/绿条）。

## 详细设计

### 架构总览

```
用户操作 (粘贴/拖拽/斜杠)
  ↓
uploadCoordinator (index.ts, 实例级单例)
  ├─ 生成 uploadId, 创建 blob URL
  ├─ 插入占位符节点 (image, data-upload-state=uploading)
  ├─ 发起 onImageUpload(file)  ← write.rs 注入的 fetch /api/upload
  │
  ├─ 成功: updateImageNode(uploadId, {src:url, 清除上传属性}) + revokeObjectURL
  └─ 失败: updateImageNode(uploadId, {data-upload-state=error, data-error-msg}) 
           + notifyRust(推 event 到 window.__tiptap_uploads)

NodeView (upload-image.ts)
  ├─ 渲染三种态 (uploading/error/done)
  └─ 按钮点击转发给 coordinator: onRetry(uploadId) / onRemove(uploadId)

Rust (write.rs)
  ├─ 轮询 window.__tiptap_uploads (500ms)
  │    ├─ 消费 events → upload_errors signal
  │    └─ 读 counts → uploads_in_flight signal
  ├─ 顶部渲染 upload_errors (多条堆叠 + ×关闭)
  └─ on_submit 检查 uploads_in_flight + 扫描 markdown 兜底 → 阻止保存
```

### 1. JS 侧：自定义 Image 扩展与 NodeView

**新文件 `libs/tiptap-editor/src/upload-image.ts`**

基于 `@tiptap/extension-image` 派生的扩展，覆盖三件事：

#### 1.1 自定义属性

继承父类的 `src/alt/title/width/height`，新增三个上传状态属性：

```typescript
addAttributes() {
  return {
    ...this.parent?.(),   // 继承 src/alt/title/width/height
    'data-upload-state': { default: null, parseHTML: el => el.getAttribute('data-upload-state') },
    'data-upload-id':    { default: null, parseHTML: el => el.getAttribute('data-upload-id') },
    'data-error-msg':    { default: null, parseHTML: el => el.getAttribute('data-error-msg') },
  }
}
```

`data-upload-state` 取值：`null`（已完成/正常图片）| `"uploading"` | `"error"`。

#### 1.2 自定义 NodeView（`addNodeView`）

NodeView 根据 `data-upload-state` 渲染三种 UI。**NodeView 只负责渲染和转发按钮点击，不直接发起上传**——上传逻辑集中在 coordinator（见 §2）。

- **`null`（已完成）**：普通 `<img>`，透传 `src`/`alt`/`width`。与原生行为一致。
- **`"uploading"`**：容器内放 `<img src="blob:url">`（本地预览）+ 绝对定位遮罩（半透明黑底 + 居中 spinner + "上传中…" 文字）。
- **`"error"`**：`<img>`（`opacity-50` 灰化）+ 遮罩（红色 ⚠ 图标 + `data-error-msg` 文案 + 两个按钮：重试 / 移除）。

NodeView 持有对 `editor` 的引用。按钮点击时：
- "重试" → 读取当前节点的 `data-upload-id`，调用 `this.options.onRetry(uploadId)`
- "移除" → 读取 `data-upload-id`，调用 `this.options.onRemove(uploadId)`

`onRetry`/`onRemove` 回调由 `index.ts` 注入（实际调用 coordinator）。

NodeView 需正确实现 Tiptap NodeView 接口：`update`（节点属性变化时重新渲染遮罩状态）、`ignoreMutation`（遮罩内的按钮点击不应被 ProseMirror 当作编辑）、`destroy`（清理 DOM）。

**属性变化重渲染机制**：当 coordinator 调用 `updateNode` 更新 `data-upload-state` 等属性时，ProseMirror 派发事务，NodeView 的 `update(node)` 被调用。NodeView 在 `update` 里比较新旧节点的 `data-upload-state`，若变化则重新渲染对应的遮罩 UI（uploading→error、error→uploading、任意→done）。这是占位符状态切换的驱动机制——NodeView 本身不持有状态，纯由节点属性驱动。

#### 1.3 Markdown 序列化

`@tiptap/markdown` 默认会丢掉非标准属性。占位符节点序列化后会变成 `![](blob:url)` 或 `![]()`。**这是可接受的**——保存拦截（见 §4）保证脏内容不进数据库，且编辑器内只对最终态（`data-upload-state=null`）的图片关心序列化正确性，此时节点属性与原生 Image 一致。

### 2. JS 侧：上传协调器（`index.ts`）

`TiptapEditorInstance` 新增一个实例级的 `uploadCoordinator`，统一管理三个上传入口（粘贴/拖拽/斜杠命令），替换现有分散在 `FileHandler.onPaste`/`onDrop` 和 `SlashCommand` 里的 `.then/.catch`。

#### 2.1 协调器职责与状态

```typescript
interface UploadEntry {
  file: File
  blobUrl: string
  fileName: string
}

class UploadCoordinator {
  private pending = new Map<string, UploadEntry>()   // uploadId → {file, blobUrl, fileName}
  constructor(
    private editor: Editor,
    private onImageUpload: (file: File) => Promise<string>,
    private notifyRust: (event: UploadEvent) => void,
  ) {}
}
```

#### 2.2 公共方法

**`insertUploading(file: File, pos?: number): void`** — 首次上传入口（粘贴/拖拽/斜杠都调它）：
```typescript
const uploadId = crypto.randomUUID()
const blobUrl = URL.createObjectURL(file)
this.pending.set(uploadId, { file, blobUrl, fileName: file.name })

// 插入占位符节点
editor.chain().focus()
  .insertContentAt(pos ?? editor.state.selection.head, {
    type: 'image',
    attrs: { src: blobUrl, 'data-upload-state': 'uploading', 'data-upload-id': uploadId }
  }).run()

this.runUpload(uploadId)
```

**`retryUpload(uploadId: string): void`** — NodeView"重试"按钮调用：
```typescript
const entry = this.pending.get(uploadId)
if (!entry) return
// 节点先转回 uploading
this.updateNode(uploadId, { 'data-upload-state': 'uploading', 'data-error-msg': null })
this.runUpload(uploadId)
```

**`removeUpload(uploadId: string): void`** — NodeView"移除"按钮调用：
```typescript
const entry = this.pending.get(uploadId)
if (!entry) return
// 删除节点（按 data-upload-id 定位）
this.removeNodeByUploadId(uploadId)
URL.revokeObjectURL(entry.blobUrl)
this.pending.delete(uploadId)
this.notifyRust({ kind: 'removed', uploadId })
```

**`removeUploadByUploadId(uploadId: string): boolean`** — Rust 侧"×关闭"提示时调用（通过 eval）。逻辑与 `removeUpload` 相同（删节点 + revoke + pending.delete + notifyRust），只是入口不同：`removeUpload` 由 NodeView 内部按钮触发，`removeUploadByUploadId` 由 Rust eval 从外部触发。返回是否成功删除（供 Rust 判断是否要清顶部提示）。

#### 2.3 私有方法

**`private async runUpload(uploadId): Promise<void>`** — 核心上传逻辑：
```typescript
const entry = this.pending.get(uploadId)
if (!entry) return
try {
  const url = await this.onImageUpload(entry.file)
  // 成功：替换 src + 清除上传属性
  this.updateNode(uploadId, {
    src: url,
    'data-upload-state': null,
    'data-upload-id': null,
    'data-error-msg': null,
  })
  URL.revokeObjectURL(entry.blobUrl)
  this.pending.delete(uploadId)
  this.notifyRust({ kind: 'success', uploadId, fileName: entry.fileName })
} catch (err) {
  const msg = this.extractErrorMessage(err)
  this.updateNode(uploadId, {
    'data-upload-state': 'error',
    'data-error-msg': msg,
  })
  this.notifyRust({ kind: 'error', uploadId, fileName: entry.fileName, errorMsg: msg })
}
```

**`private updateNode(uploadId, attrs): void`** — 按 `data-upload-id` 定位节点并更新属性。上传完成时光标早已移走，不能依赖选区，必须遍历文档回查：
```typescript
let targetPos: number | null = null
editor.state.doc.descendants((node, pos) => {
  if (node.type.name === 'image' && node.attrs['data-upload-id'] === uploadId) {
    targetPos = pos
    return false  // 停止遍历
  }
  return true
})
if (targetPos !== null) {
  const tr = editor.state.tr.setNodeMarkup(targetPos, undefined, { ...node.attrs, ...attrs })
  editor.view.dispatch(tr)
}
```

**`private removeNodeByUploadId(uploadId): void`** — 类似 `updateNode` 定位后用 `tr.delete(pos, pos + nodeSize)`。

**`private extractErrorMessage(err): string`** — 从错误对象提取服务端中文消息：
- 若 err 是 Error 且 message 以 `"Upload failed: "` 开头（`write.rs` 的旧格式），尝试回退到通用提示
- 否则直接用 message
- **注意**：见 §5 的 `write.rs` fetch 改造，改造后 err.message 会直接是服务端中文（如"文件超过大小限制"），此处只需透传

#### 2.4 三个上传入口的改造

| 入口 | 改动 |
|------|------|
| `FileHandler.onPaste` | 改为 `coordinator.insertUploading(file)`（无 pos，插入选区） |
| `FileHandler.onDrop` | 改为 `coordinator.insertUploading(file, pos)`（用 onDrop 给的 pos） |
| `SlashCommand` 上传图片命令 | 改为 `coordinator.insertUploading(file)` |

三处原本的 `.then(setImage).catch(console.error)` 全部删除，统一走 coordinator。

### 3. JS→Rust 状态通道

#### 3.1 全局对象结构

```javascript
window.__tiptap_uploads = {
  // 新事件队列：Rust 消费后清空
  events: [
    { kind: 'error', uploadId: 'uuid1', fileName: 'cat.png', errorMsg: '文件超过大小限制', ts: 1719... },
    { kind: 'success', uploadId: 'uuid2', fileName: 'dog.png', ts: 1719... },
    { kind: 'removed', uploadId: 'uuid1', ts: 1719... },
  ],
  // 实时计数（始终反映当前编辑器内占位符状态）
  counts: { uploading: 2, error: 1 }
}
```

- `events`：追加型队列。coordinator 每次 `runUpload` 成功/失败、`removeUpload` 时追加一个 event。Rust 轮询时读取并清空（`u.events = []`）。
- `counts`：coordinator 在每次状态变化后重新计算（遍历 `editor.state.doc` 统计 `data-upload-state` 为 `uploading`/`error` 的节点数），写入 `counts`。Rust 每次轮询直接读当前值。

#### 3.2 `notifyRust(event)` 实现

```typescript
private notifyRust(event: UploadEvent) {
  if (!window.__tiptap_uploads) {
    window.__tiptap_uploads = { events: [], counts: { uploading: 0, error: 0 } }
  }
  window.__tiptap_uploads.events.push({ ...event, ts: Date.now() })
  // 重新计算 counts
  let uploading = 0, error = 0
  this.editor.state.doc.descendants((node) => {
    const state = node.attrs['data-upload-state']
    if (state === 'uploading') uploading++
    else if (state === 'error') error++
    return true
  })
  window.__tiptap_uploads.counts = { uploading, error }
}
```

### 4. Rust 侧：轮询消费与渲染（`write.rs`）

#### 4.1 新增 signal

```rust
#[derive(Clone, Copy, Default)]
struct UploadsInFlight { uploading: u32, error: u32 }

// 当前进行中的上传计数（保存拦截用）
let mut uploads_in_flight = use_signal(UploadsInFlight::default);

// 顶部堆叠的失败提示（用户手动关闭）
struct UploadErrorEntry { id: String, file_name: String, message: String }
let mut upload_errors: Signal<Vec<UploadErrorEntry>> = use_signal(Vec::new);
```

#### 4.2 轮询 effect

复用现有 `spawn_local` 轮询模式（参考 `write.rs:210-238` 的 `__tiptap_ready` 轮询）。新增独立 `use_future`，500ms 间隔：

```rust
use_future(move || async move {
    let mut seen_error_ids: HashSet<String> = HashSet::new();
    loop {
        sleep(500ms);
        #[cfg(target_arch = "wasm32")]
        {
            let snapshot = js_sys::eval(r#"
                (function() {
                    var u = window.__tiptap_uploads;
                    if (!u) return null;
                    var events = u.events || [];
                    u.events = [];
                    return JSON.stringify({ events: events, counts: u.counts || {uploading:0,error:0} });
                })()
            "#).ok().and_then(|v| v.as_string());

            if let Some(json) = snapshot {
                if let Ok(parsed) = serde_json::from_str::<UploadSnapshot>(&json) {
                    // 1. 消费 events
                    for ev in parsed.events {
                        match ev.kind.as_str() {
                            "error" => {
                                if !seen_error_ids.contains(&ev.uploadId) {
                                    seen_error_ids.insert(ev.uploadId.clone());
                                    upload_errors.write().push(UploadErrorEntry {
                                        id: ev.uploadId,
                                        file_name: ev.fileName,
                                        message: ev.errorMsg,
                                    });
                                }
                            }
                            "success" | "removed" => {
                                // 该 id 不再是失败态，从顶部提示移除
                                seen_error_ids.remove(&ev.uploadId);
                                upload_errors.write().retain(|e| e.id != ev.uploadId);
                            }
                            _ => {}
                        }
                    }
                    // 2. 更新 counts
                    uploads_in_flight.set(UploadsInFlight {
                        uploading: parsed.counts.uploading,
                        error: parsed.counts.error,
                    });
                }
            }
        }
    }
});
```

**counts 同步的重要性**：当用户在编辑器内点"移除"删掉失败占位符，coordinator 发 `removed` event + 重算 counts。Rust 消费 `removed` event 时从 `upload_errors` 移除对应条目——**保证编辑器内卡片和顶部提示同步**，不会出现"占位符删了但顶部提示还在"的孤儿状态。

注：`success` event 在 Rust 侧也会走 `seen_error_ids.remove + upload_errors.retain` 分支，但 success 的 id 从未进过 `seen_error_ids`/`upload_errors`（只有 error 才进），所以这是无害的多余操作，保留统一处理逻辑即可。

#### 4.3 顶部提示渲染

在 `write.rs` 现有 `load_error()`/`error()`/`success()` 提示区附近（约 line 453-469），新增上传错误区：

```rust
for err in upload_errors.read().iter() {
    div {
        class: "flex-shrink-0 flex items-center justify-between px-4 py-2 
                bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 
                rounded-xl text-sm border border-red-100 dark:border-red-900/30 mb-2",
        span { "图片上传失败: {err.file_name} — {err.message}" }
        button {
            class: "ml-3 text-red-400 hover:text-red-600 cursor-pointer",
            onclick: move |_| {
                // 关闭提示，同时删除编辑器内的失败占位符（避免孤儿）
                let _ = js_sys::eval(&format!(
                    "(function(){{var e=window.TiptapEditor&&window.TiptapEditor._instances&&window.TiptapEditor._instances.get('tiptap-editor');if(e&&e.removeUploadByUploadId){{e.removeUploadByUploadId({:?});}}}})()",
                    err.id
                ));
                upload_errors.write().retain(|e| e.id != err.id);
            },
            "×"
        }
    }
}
```

**×关闭同时删除失败占位符**：用户点×的语义是"清掉这个失败"，保留编辑器内的红色卡片会变成孤儿（顶部无提示了但编辑器里还挂着）。通过 eval 调 `removeUploadByUploadId` 删除占位符 + revoke blob URL + 发 `removed` event。

#### 4.4 保存拦截（双重防护）

**第一道：counts 检查**（主提示）

`on_submit`（约 line 247）开头，读 markdown 前加检查：

```rust
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
```

**第二道：markdown 兜底扫描**（防御性）

500ms 轮询有窗口期（用户刚传完、counts 还没更新就点保存）。拿到 markdown 后扫描是否含残留的占位符标记：

```rust
// 拿到 md 后
if md.contains("blob:") || md.contains("data-upload-state") {
    error.set(Some("检测到未完成上传的图片，请处理后保存".to_string()));
    return;
}
```

这两道防护共同保证：脏内容（blob URL 或带上传状态的节点）不会写入数据库。

### 5. `write.rs` fetch 改造（透传服务端错误）

当前 `onImageUpload` 的 fetch 在非 2xx 时丢弃了服务端的中文错误：

```javascript
// 当前（丢弃错误体）
if (!response.ok) {
    throw new Error('Upload failed: ' + response.status);
}
```

改为读取错误响应体：

```javascript
// 改造后
if (!response.ok) {
    // 读取服务端返回的中文错误（{"success":false,"error":"文件超过大小限制"}）
    // 服务端所有失败路径都返回此 JSON 格式（见 upload.rs），解析可靠
    const data = await response.json().catch(() => null);
    if (data && data.error) {
        throw new Error(data.error);
    }
    // 响应体不是 JSON（极端情况，如反向代理错误页），退回状态码
    throw new Error('上传失败: ' + response.status);
}
```

改造后，coordinator 的 `extractErrorMessage` 直接透传即可拿到"文件超过大小限制"等中文消息。

## 实现边界与清单

### JS 侧（`libs/tiptap-editor/src/`）

| 文件 | 改动 |
|------|------|
| `upload-image.ts` | **新建**：自定义 Image 扩展（继承父类属性 + 三个上传属性 + NodeView） |
| `index.ts` | 替换 `Image.configure(...)` 为自定义扩展；新增 `UploadCoordinator` 类；`FileHandler.onPaste/onDrop`、`SlashCommand` 统一走 `coordinator.insertUploading`；实现 `notifyRust` + `removeUploadByUploadId` 暴露给 Rust |
| `slash-command.ts` | 上传图片命令的 `.then/.catch` 改为 `coordinator.insertUploading(file)` |
| `style.css` | 新增 NodeView 三种态的样式（遮罩、spinner、错误卡片、重试/移除按钮） |

### Rust 侧（`src/pages/admin/write.rs`）

| 改动点 | 说明 |
|--------|------|
| 新增 signal | `uploads_in_flight`、`upload_errors` |
| 新增轮询 effect | 500ms 消费 `window.__tiptap_uploads` |
| 顶部提示渲染 | 多条堆叠 + ×关闭（同时删占位符） |
| `on_submit` 拦截 | counts 检查 + markdown 兜底扫描 |
| `onImageUpload` fetch 改造 | 读取非 2xx 响应体的 `error` 字段 |

### 不做的事

- 不引入 wasm-bindgen 导出机制（保持 eval 桥接一致性）
- 不做 toast/自动消失提示（保持与现有静态提示风格一致）
- 不引入图片编辑/裁剪能力
- 不改动服务端 `upload.rs`（它的错误响应格式已满足需求，只是前端没读）
- 不处理编辑模式（`WriteEdit`）下的旧文章占位符回填——旧文章的图片都是 `data-upload-state=null` 的正常图片，不涉及上传态

## 验收标准

- [ ] 粘贴/拖拽/斜杠命令上传图片时，编辑器立即显示本地预览图 + "上传中…"遮罩
- [ ] 上传成功后，遮罩消失，图片 src 替换为服务端 URL，无光标跳动
- [ ] 上传失败时，占位符变红，显示服务端中文错误（如"文件超过大小限制"）
- [ ] 失败占位符上有"重试"和"移除"按钮，点重试用原文件重新上传，点移除删除节点
- [ ] 上传失败时页面顶部出现堆叠提示，显示文件名 + 错误原因 + ×关闭
- [ ] 多张图片同时失败时，顶部提示多条堆叠，逐条可关闭
- [ ] 在编辑器内移除失败占位符，顶部对应提示同步消失
- [ ] 点顶部×关闭，编辑器内对应失败占位符同步删除
- [ ] 有 uploading 占位符时点保存，被阻止并提示"有 N 张图片正在上传"
- [ ] 有 error 占位符时点保存，被阻止并提示"有 N 张图片上传失败"
- [ ] markdown 兜底扫描：即使轮询窗口期漏判，blob: 残留也能被拦下
- [ ] 上传超大文件（>5MB）时错误提示为"文件超过大小限制"而非"Upload failed: 413"

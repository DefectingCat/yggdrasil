# 编辑器图片上传占位符与失败提示 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在文章编辑器上传图片时显示本地预览占位符 + loading 遮罩，上传失败显示错误卡片（带重试/移除）+ 顶部堆叠提示，并在有未完成上传时阻止保存。

**Architecture:** JS 主导——自定义 Tiptap Image 扩展（NodeView 承载上传状态属性）+ UploadCoordinator（集中上传逻辑，按 upload-id 定位节点更新属性）。Rust 侧通过 500ms 轮询 `window.__tiptap_uploads` 全局对象消费上传事件与计数，驱动顶部提示和保存拦截。

**Tech Stack:** TypeScript（Tiptap 3.25.0 NodeView API）、Rust/Dioxus 0.7（eval 桥接 + 信号）、Tailwind CSS

**前置 spec:** `docs/superpowers/specs/2026-06-22-upload-placeholder-design.md`

**关键实现约定（贯穿所有任务）：**
- NodeView 用 plain class 实现（参考 `@tiptap/core` 的 `ResizableNodeView`），不继承 NodeView 基类，不使用 React/Vue。
- uploadId 生成用 `crypto.randomUUID()` 带 fallback：`crypto.randomUUID?.() ?? Math.random().toString(36).slice(2)`（兼容非安全上下文）。
- `onImageUpload: (file: File) => Promise<string>` 的契约不变（resolve 服务端 URL），coordinator 内部消费它。

---

## 文件结构

| 文件 | 责任 | 操作 |
|------|------|------|
| `libs/tiptap-editor/src/upload-coordinator.ts` | UploadCoordinator 类：管理 pending Map、发起上传、按 id 定位更新/删除节点、维护 `window.__tiptap_uploads` | 新建 |
| `libs/tiptap-editor/src/upload-image.ts` | 自定义 Image 扩展（继承属性 + 三个上传属性 + NodeView 类） | 新建 |
| `libs/tiptap-editor/src/index.ts` | 替换 Image 为自定义扩展；FileHandler/SlashCommand 走 coordinator；暴露 removeUploadByUploadId | 改 |
| `libs/tiptap-editor/src/slash-command.ts` | 上传命令调 coordinator.insertUploading | 改 |
| `libs/tiptap-editor/src/style.css` | NodeView 三种态样式（遮罩/spinner/错误卡片/按钮） | 改 |
| `src/pages/admin/write.rs` | signal、轮询 effect、顶部提示、保存拦截、fetch 改造 | 改 |

任务顺序：先建 JS 侧（coordinator → upload-image 扩展 → index 接线 → slash-command → 样式），再做 Rust 侧（signal/轮询 → 提示渲染 → 保存拦截 → fetch 改造），最后整体验证。

---

## Task 1: UploadCoordinator 基础类（pending Map + insertUploading + removeUpload）

**Files:**
- Create: `libs/tiptap-editor/src/upload-coordinator.ts`

此任务建立 coordinator 的核心：持有 pending Map、生成 uploadId、创建 blob URL、按 id 定位节点删除。上传发起逻辑（runUpload）在 Task 2 加入，notifyRust 在 Task 3 加入。先让 coordinator 能管理 pending 状态和节点删除。

- [ ] **Step 1: 创建 upload-coordinator.ts 骨架与类型**

创建 `libs/tiptap-editor/src/upload-coordinator.ts`：

```typescript
import type { Editor } from '@tiptap/core'

/** pending 上传条目：保留 File 供重试，blobUrl 供本地预览。 */
interface UploadEntry {
  file: File
  blobUrl: string
  fileName: string
}

/** coordinator 推给 Rust 的事件（通过 window.__tiptap_uploads.events）。 */
export interface UploadEvent {
  kind: 'error' | 'success' | 'removed'
  uploadId: string
  fileName: string
  errorMsg?: string
  ts: number
}

/** 生成 uploadId，兼容非安全上下文（无 crypto.randomUUID 时）。 */
function genUploadId(): string {
  return crypto.randomUUID?.() ?? Math.random().toString(36).slice(2)
}

/**
 * 上传协调器：集中管理图片上传生命周期。
 *
 * 职责：
 * - 生成 uploadId、创建 blob URL、插入占位符节点
 * - 发起上传，成功更新节点 src、失败转 error 态
 * - 按 upload-id 定位节点更新/删除（上传完成时光标早已移走）
 * - 维护 window.__tiptap_uploads 供 Rust 轮询
 *
 * pending Map 保留 File 对象直到上传成功或显式移除，支持无限重试。
 */
export class UploadCoordinator {
  private pending = new Map<string, UploadEntry>()

  constructor(
    private editor: Editor,
    private onImageUpload: (file: File) => Promise<string>,
  ) {}

  /**
   * 插入上传中占位符并发起上传。
   * pos 省略时插入当前选区。
   */
  insertUploading(file: File, pos?: number): void {
    const uploadId = genUploadId()
    const blobUrl = URL.createObjectURL(file)
    this.pending.set(uploadId, { file, blobUrl, fileName: file.name })

    this.editor.chain().focus().insertContentAt(pos ?? this.editor.state.selection.head, {
      type: 'image',
      attrs: {
        src: blobUrl,
        'data-upload-state': 'uploading',
        'data-upload-id': uploadId,
      },
    }).run()
  }

  /** 按 uploadId 删除节点（revoke blob、清 pending）。NodeView 移除按钮 / Rust ×关闭 共用。 */
  removeUpload(uploadId: string): boolean {
    const entry = this.pending.get(uploadId)
    if (!entry) return false
    this.removeNodeByUploadId(uploadId)
    URL.revokeObjectURL(entry.blobUrl)
    this.pending.delete(uploadId)
    return true
  }

  /** 按 uploadId 在文档中定位节点并删除。 */
  private removeNodeByUploadId(uploadId: string): void {
    let targetPos: number | null = null
    let nodeSize = 0
    this.editor.state.doc.descendants((node, pos) => {
      if (node.type.name === 'image' && node.attrs['data-upload-id'] === uploadId) {
        targetPos = pos
        nodeSize = node.nodeSize
        return false
      }
      return true
    })
    if (targetPos !== null) {
      const tr = this.editor.state.tr.delete(targetPos, targetPos + nodeSize)
      this.editor.view.dispatch(tr)
    }
  }

  /** 按 uploadId 定位节点并合并更新属性。 */
  private updateNodeAttrs(uploadId: string, attrs: Record<string, unknown>): void {
    let targetPos: number | null = null
    let oldAttrs: Record<string, unknown> | null = null
    this.editor.state.doc.descendants((node, pos) => {
      if (node.type.name === 'image' && node.attrs['data-upload-id'] === uploadId) {
        targetPos = pos
        oldAttrs = node.attrs
        return false
      }
      return true
    })
    if (targetPos !== null && oldAttrs) {
      const tr = this.editor.state.tr.setNodeMarkup(
        targetPos,
        undefined,
        { ...oldAttrs, ...attrs },
      )
      this.editor.view.dispatch(tr)
    }
  }

  /** pending Map 仅供内部/测试访问。 */
  hasPending(uploadId: string): boolean {
    return this.pending.has(uploadId)
  }
}
```

- [ ] **Step 2: 验证 TypeScript 编译**

Run: `cd libs/tiptap-editor && npx tsc --noEmit src/upload-coordinator.ts 2>&1 | head -20`
Expected: 无错误（可能有 "cannot find module @tiptap/core" 若 tsc 单文件不解析 node_modules，改用 `npx tsc --noEmit -p .` 若有 tsconfig；若无 tsconfig，跳过此步靠 vite build 兜底）。实际项目无 tsconfig.json，依赖 vite 的 esbuild 转译，此步改为 Step 3 的 vite build 验证。

注：`libs/tiptap-editor/` 无独立 tsconfig，类型检查依赖最终 `vite build`。此文件此刻未被 import，vite build 不会包含它——类型错误要到 Task 4 接线后才暴露。**此步仅确认文件创建成功**。

Run: `test -f libs/tiptap-editor/src/upload-coordinator.ts && echo "created"`
Expected: `created`

- [ ] **Step 3: Commit**

```bash
git add libs/tiptap-editor/src/upload-coordinator.ts
git commit -m "feat(editor): add UploadCoordinator skeleton with pending map and node removal"
```

---

## Task 2: coordinator 的上传逻辑（runUpload + retryUpload）

**Files:**
- Modify: `libs/tiptap-editor/src/upload-coordinator.ts`

加入核心上传发起逻辑和重试。Task 1 建好的 `insertUploading` 此时调用 `runUpload`，重试从 pending 取回 File 重跑。

- [ ] **Step 1: 在 insertUploading 末尾追加 runUpload 调用**

在 `upload-coordinator.ts` 的 `insertUploading` 方法 `.run()` 之后追加：

```typescript
    this.runUpload(uploadId)
```

完整方法变为：
```typescript
  insertUploading(file: File, pos?: number): void {
    const uploadId = genUploadId()
    const blobUrl = URL.createObjectURL(file)
    this.pending.set(uploadId, { file, blobUrl, fileName: file.name })

    this.editor.chain().focus().insertContentAt(pos ?? this.editor.state.selection.head, {
      type: 'image',
      attrs: {
        src: blobUrl,
        'data-upload-state': 'uploading',
        'data-upload-id': uploadId,
      },
    }).run()

    this.runUpload(uploadId)
  }
```

- [ ] **Step 2: 添加 runUpload 私有方法**

在 `UploadCoordinator` 类内（`insertUploading` 之后）添加：

```typescript
  /** 核心上传逻辑：成功更新 src + 清上传属性，失败转 error 态。 */
  private async runUpload(uploadId: string): Promise<void> {
    const entry = this.pending.get(uploadId)
    if (!entry) return
    try {
      const url = await this.onImageUpload(entry.file)
      // 成功：替换 src，清除上传状态属性
      this.updateNodeAttrs(uploadId, {
        src: url,
        'data-upload-state': null,
        'data-upload-id': null,
        'data-error-msg': null,
      })
      URL.revokeObjectURL(entry.blobUrl)
      this.pending.delete(uploadId)
    } catch (err) {
      const msg = this.extractErrorMessage(err)
      this.updateNodeAttrs(uploadId, {
        'data-upload-state': 'error',
        'data-error-msg': msg,
      })
    }
  }
```

- [ ] **Step 3: 添加 retryUpload 公开方法**

在 `runUpload` 之后添加（NodeView 重试按钮调用）：

```typescript
  /** 重试：从 pending 取回原 File，节点转回 uploading，重跑上传。 */
  retryUpload(uploadId: string): void {
    const entry = this.pending.get(uploadId)
    if (!entry) return
    this.updateNodeAttrs(uploadId, {
      'data-upload-state': 'uploading',
      'data-error-msg': null,
    })
    this.runUpload(uploadId)
  }
```

- [ ] **Step 4: 添加 extractErrorMessage 私有方法**

在 `updateNodeAttrs` 之后添加：

```typescript
  /** 从错误对象提取消息。改造后的 fetch 直接抛服务端中文（如"文件超过大小限制"）。 */
  private extractErrorMessage(err: unknown): string {
    if (err instanceof Error) return err.message
    return String(err)
  }
```

- [ ] **Step 5: Commit**

```bash
git add libs/tiptap-editor/src/upload-coordinator.ts
git commit -m "feat(editor): add runUpload and retryUpload to UploadCoordinator"
```

---

## Task 3: coordinator 的 Rust 通知（notifyRust + window 全局）

**Files:**
- Modify: `libs/tiptap-editor/src/upload-coordinator.ts`

加入 `window.__tiptap_uploads` 全局对象维护逻辑。coordinator 在每次状态变化后追加 event 并重算 counts。

- [ ] **Step 1: 添加 notifyRust 私有方法**

在 `extractErrorMessage` 之后添加：

```typescript
  /**
   * 追加事件到 window.__tiptap_uploads.events，并重算 counts。
   * Rust 侧 500ms 轮询消费 events 并读取 counts。
   */
  private notifyRust(event: Omit<UploadEvent, 'ts'>): void {
    const w = window as unknown as { __tiptap_uploads?: { events: UploadEvent[]; counts: { uploading: number; error: number } } }
    if (!w.__tiptap_uploads) {
      w.__tiptap_uploads = { events: [], counts: { uploading: 0, error: 0 } }
    }
    w.__tiptap_uploads.events.push({ ...event, ts: Date.now() })

    // 重算 counts：遍历文档统计当前 uploading/error 占位符
    let uploading = 0
    let error = 0
    this.editor.state.doc.descendants((node) => {
      const state = node.attrs['data-upload-state']
      if (state === 'uploading') uploading++
      else if (state === 'error') error++
      return true
    })
    w.__tiptap_uploads.counts = { uploading, error }
  }
```

- [ ] **Step 2: 在 runUpload 成功分支追加 notifyRust**

`runUpload` 成功分支（`this.pending.delete(uploadId)` 之后）追加：

```typescript
      this.notifyRust({ kind: 'success', uploadId, fileName: entry.fileName })
```

- [ ] **Step 3: 在 runUpload 失败分支追加 notifyRust**

`runUpload` catch 分支（`updateNodeAttrs(...error...)` 之后）追加：

```typescript
      this.notifyRust({ kind: 'error', uploadId, fileName: entry.fileName, errorMsg: msg })
```

- [ ] **Step 4: 在 removeUpload 追加 notifyRust**

`removeUpload` 方法（`this.pending.delete(uploadId)` 之后，`return true` 之前）追加：

```typescript
    this.notifyRust({ kind: 'removed', uploadId, fileName: entry.fileName })
```

- [ ] **Step 5: Commit**

```bash
git add libs/tiptap-editor/src/upload-coordinator.ts
git commit -m "feat(editor): maintain window.__tiptap_uploads for Rust polling"
```

---

## Task 4: 自定义 Image 扩展与 NodeView 类（upload-image.ts）

**Files:**
- Create: `libs/tiptap-editor/src/upload-image.ts`

核心：自定义 Image 扩展继承父类属性 + 三个上传属性 + NodeView 类渲染三种态。NodeView 是 plain class，参考 `@tiptap/core` 的 ResizableNodeView 形态。

- [ ] **Step 1: 创建 upload-image.ts — 自定义扩展与属性**

创建 `libs/tiptap-editor/src/upload-image.ts`：

```typescript
import { Image, mergeAttributes, type Editor } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'

/** NodeView 按钮点击回调注入接口。 */
export interface UploadNodeViewCallbacks {
  onRetry: (uploadId: string) => void
  onRemove: (uploadId: string) => void
}

/**
 * 上传图片占位符 NodeView：plain class，实现 ProseMirror NodeView 接口。
 *
 * 根据 data-upload-state 渲染三种态：
 * - null：普通 img
 * - uploading：img（本地 blob 预览）+ 遮罩（spinner + "上传中…"）
 * - error：img（灰化）+ 遮罩（⚠ + 错误文案 + 重试/移除按钮）
 *
 * NodeView 纯渲染，不发起上传——按钮点击转发给注入的 callbacks（实际是 coordinator）。
 * 属性变化时 ProseMirror 调 update(node)，NodeView 比较新旧 data-upload-state 重渲染遮罩。
 */
class UploadImageNodeView {
  private editor: Editor
  private node: PMNode
  private getPos: () => number | undefined
  private callbacks: UploadNodeViewCallbacks

  private container: HTMLDivElement
  private img: HTMLImageElement
  private overlay: HTMLDivElement | null = null

  constructor(opts: {
    node: PMNode
    editor: Editor
    getPos: () => number | undefined
    HTMLAttributes: Record<string, unknown>
    callbacks: UploadNodeViewCallbacks
  }) {
    this.node = opts.node
    this.editor = opts.editor
    this.getPos = opts.getPos
    this.callbacks = opts.callbacks

    this.container = document.createElement('div')
    this.container.classList.add('upload-image-container')

    this.img = document.createElement('img')
    this.img.draggable = false
    const merged = mergeAttributes(opts.HTMLAttributes)
    Object.entries(merged).forEach(([k, v]) => {
      if (v != null) this.img.setAttribute(k, String(v))
    })
    const src = this.node.attrs.src
    if (src != null) this.img.src = src
    this.container.appendChild(this.img)

    this.renderOverlay()
  }

  get dom(): HTMLElement {
    return this.container
  }

  get contentDOM(): HTMLElement | null {
    return null
  }

  /** ProseMirror 调用：节点属性变化时重渲染遮罩。返回 false 拒绝非同类节点。 */
  update(node: PMNode): boolean {
    if (node.type !== this.node.type) return false
    const oldState = this.node.attrs['data-upload-state']
    const newState = node.attrs['data-upload-state']
    const oldSrc = this.node.attrs.src
    const newSrc = node.attrs.src
    this.node = node
    if (oldSrc !== newSrc && newSrc != null) {
      this.img.src = newSrc
    }
    if (oldState !== newState || this.overlay === null) {
      this.renderOverlay()
    }
    return true
  }

  /** 遮罩内按钮点击不被 ProseMirror 当编辑，避免误触发事务。 */
  ignoreMutation(): boolean {
    return true
  }

  /** 事件不被编辑器 stopEvent 拦截（按钮点击要响应）。 */
  stopEvent(event: Event): boolean {
    return false
  }

  /** 根据 data-upload-state 渲染遮罩。null 时移除遮罩。 */
  private renderOverlay(): void {
    if (this.overlay) {
      this.overlay.remove()
      this.overlay = null
    }
    const state = this.node.attrs['data-upload-state']
    if (state == null) {
      this.container.classList.remove('is-uploading', 'is-error')
      return
    }
    this.overlay = document.createElement('div')
    this.overlay.classList.add('upload-image-overlay')
    if (state === 'uploading') {
      this.container.classList.add('is-uploading')
      this.container.classList.remove('is-error')
      this.overlay.innerHTML =
        '<div class="upload-spinner"></div><div class="upload-overlay-text">上传中…</div>'
    } else if (state === 'error') {
      this.container.classList.add('is-error')
      this.container.classList.remove('is-uploading')
      const msg = this.node.attrs['data-error-msg'] || '上传失败'
      this.overlay.innerHTML =
        '<div class="upload-error-icon">⚠</div>' +
        '<div class="upload-error-msg"></div>' +
        '<div class="upload-error-actions">' +
        '<button type="button" class="upload-btn upload-btn-retry">重试</button>' +
        '<button type="button" class="upload-btn upload-btn-remove">移除</button>' +
        '</div>'
      const msgEl = this.overlay.querySelector('.upload-error-msg') as HTMLElement
      msgEl.textContent = msg
      const uploadId = this.node.attrs['data-upload-id'] as string | null
      this.overlay.querySelector('.upload-btn-retry')?.addEventListener('click', (e) => {
        e.preventDefault()
        if (uploadId) this.callbacks.onRetry(uploadId)
      })
      this.overlay.querySelector('.upload-btn-remove')?.addEventListener('click', (e) => {
        e.preventDefault()
        if (uploadId) this.callbacks.onRemove(uploadId)
      })
    }
    this.container.appendChild(this.overlay)
  }

  destroy(): void {
    this.overlay?.remove()
    this.overlay = null
    this.container.remove()
  }
}

/** coordinator 引用（由 index.ts 在创建 editor 前注入）。 */
let coordinatorRef: { retryUpload: (id: string) => void; removeUpload: (id: string) => boolean } | null = null

/** index.ts 注入 coordinator，供 NodeView 的 onRetry/onRemove 调用。 */
export function setUploadCoordinator(c: typeof coordinatorRef): void {
  coordinatorRef = c
}

/**
 * 自定义 Image 扩展：继承父类属性，加三个上传状态属性，用自定义 NodeView。
 */
export const UploadImage = Image.configure({ allowBase64: true }).extend({
  addAttributes() {
    return {
      ...this.parent?.(),
      'data-upload-state': {
        default: null,
        parseHTML: (el) => el.getAttribute('data-upload-state'),
        renderHTML: (attrs) => {
          const v = attrs['data-upload-state']
          return v == null ? {} : { 'data-upload-state': v }
        },
      },
      'data-upload-id': {
        default: null,
        parseHTML: (el) => el.getAttribute('data-upload-id'),
        renderHTML: (attrs) => {
          const v = attrs['data-upload-id']
          return v == null ? {} : { 'data-upload-id': v }
        },
      },
      'data-error-msg': {
        default: null,
        parseHTML: (el) => el.getAttribute('data-error-msg'),
        renderHTML: (attrs) => {
          const v = attrs['data-error-msg']
          return v == null ? {} : { 'data-error-msg': v }
        },
      },
    }
  },

  addNodeView() {
    return ({ node, getPos, HTMLAttributes, editor }) => {
      return new UploadImageNodeView({
        node,
        editor,
        getPos,
        HTMLAttributes,
        callbacks: {
          onRetry: (id) => coordinatorRef?.retryUpload(id),
          onRemove: (id) => coordinatorRef?.removeUpload(id),
        },
      })
    }
  },
})
```

- [ ] **Step 2: Commit**

```bash
git add libs/tiptap-editor/src/upload-image.ts
git commit -m "feat(editor): add custom Image extension with upload-state NodeView"
```

---

## Task 5: index.ts 接线（coordinator + 自定义 Image + 移除旧 onPaste/onDrop 逻辑）

**Files:**
- Modify: `libs/tiptap-editor/src/index.ts`

替换 Image 为自定义扩展；实例化 coordinator 并注入给 NodeView；FileHandler.onPaste/onDrop 改走 coordinator；暴露 removeUploadByUploadId 供 Rust 调用。

- [ ] **Step 1: 添加 import**

在 `index.ts` 顶部 import 区（`import './style.css'` 之前）添加：

```typescript
import { UploadCoordinator } from './upload-coordinator'
import { UploadImage, setUploadCoordinator } from './upload-image'
```

- [ ] **Step 2: 在 TiptapEditorInstance 添加 coordinator 私有字段**

在 `private toggleButton: HTMLButtonElement | null = null`（第 29 行）之后添加：

```typescript
  private coordinator: UploadCoordinator | null = null
```

- [ ] **Step 3: 替换 Image 为 UploadImage，在 editor 创建后实例化 coordinator**

将 `extensions` 数组中的 `Image.configure({ allowBase64: true }),`（第 84 行）替换为：

```typescript
        UploadImage,
```

然后在 `this.editor = new Editor({...})` 赋值之后（第 141 行 `})` 之后，即 `onUpdate`/`onBlur` 等配置结束、Editor 创建完成的 `})` 闭合处之后）添加：

```typescript
    // 创建上传协调器，注入给 NodeView 的 onRetry/onRemove
    if (this.options.onImageUpload) {
      this.coordinator = new UploadCoordinator(this.editor, this.options.onImageUpload)
      setUploadCoordinator(this.coordinator)
    }
```

- [ ] **Step 4: 改写 FileHandler.onPaste 走 coordinator**

将 `onPaste`（第 93-105 行）替换为：

```typescript
          onPaste: (editor, files) => {
            if (this.coordinator) {
              files.forEach((file) => this.coordinator!.insertUploading(file))
            }
          },
```

- [ ] **Step 5: 改写 FileHandler.onDrop 走 coordinator**

将 `onDrop`（第 107-123 行）替换为：

```typescript
          onDrop: (editor, files, pos) => {
            if (this.coordinator) {
              files.forEach((file) => this.coordinator!.insertUploading(file, pos))
            }
          },
```

- [ ] **Step 6: 添加 removeUploadByUploadId 公开方法**

在 `destroy()` 方法（第 238 行附近）之前添加：

```typescript
  /** Rust 侧"×关闭"提示时调用（通过 eval）。返回是否成功删除。 */
  removeUploadByUploadId(uploadId: string): boolean {
    return this.coordinator?.removeUpload(uploadId) ?? false
  }
```

- [ ] **Step 7: 在 destroy 中清理 coordinator**

在 `destroy()` 方法（第 238-246 行）的 `this.editor = null` 之后添加：

```typescript
    this.coordinator = null
```

- [ ] **Step 8: 构建验证**

Run: `make build-editor-incremental`
Expected: `✓ built in ...`，无错误。若有 TS 错误，修正后再构建。

- [ ] **Step 9: Commit**

```bash
git add libs/tiptap-editor/src/index.ts
git commit -m "feat(editor): wire UploadCoordinator and custom Image in TiptapEditorInstance"
```

---

## Task 6: slash-command.ts 上传命令走 coordinator

**Files:**
- Modify: `libs/tiptap-editor/src/slash-command.ts`

当前斜杠"上传图片"命令直接调 `onImageUpload(file).then(setImage).catch(console.error)`。改为调 coordinator.insertUploading。但 slash-command 扩展拿不到 coordinator——通过新增 `onInsertUploading` option 注入。

- [ ] **Step 1: 扩展 SlashCommandOptions，加 onInsertUploading**

在 `slash-command.ts` 的 `SlashCommandOptions` 接口（约第 19-21 行）改为：

```typescript
export interface SlashCommandOptions {
  onImageUpload?: (file: File) => Promise<string>
  /** 由 index.ts 注入：直接调 coordinator.insertUploading（走占位符 + 上传）。 */
  onInsertUploading?: (file: File) => void
}
```

- [ ] **Step 2: addOptions 返回新字段**

将 `addOptions()` 的返回（约第 33-35 行）改为：

```typescript
  addOptions() {
    return {
      onImageUpload: undefined,
      onInsertUploading: undefined,
    }
  },
```

- [ ] **Step 3: 上传图片命令改调 onInsertUploading**

在 `addProseMirrorPlugins` 内，"上传图片"命令的 command（约第 119-137 行）改为：

```typescript
      {
        title: '上传图片',
        description: '从本地选择并上传图片',
        icon: '📤',
        command: ({ editor, range }) => {
          editor.chain().focus().deleteRange(range).run()
          const input = document.createElement('input')
          input.type = 'file'
          input.accept = 'image/jpeg,image/png,image/gif,image/webp'
          input.addEventListener('change', () => {
            const file = input.files?.[0]
            if (!file) return
            // 优先走 coordinator（占位符 + 上传），否则退回直接上传（无占位符）
            if (this.options.onInsertUploading) {
              this.options.onInsertUploading(file)
            } else if (uploadFn) {
              uploadFn(file)
                .then((url) => editor.chain().focus().setImage({ src: url }).run())
                .catch((err) => {
                  const msg = err instanceof Error ? err.message : String(err)
                  console.error('[SlashCommand] Upload failed:', msg)
                })
            }
          })
          input.click()
        },
      },
```

注意：`uploadFn` 变量（第 38 行 `const uploadFn = this.options.onImageUpload`）保留，作为 "是否显示上传图片命令" 的判断条件（下一行的 `if (uploadFn)`）。新的 `onInsertUploading` 是 coordinator 走向，`uploadFn` 仅用于决定命令可见性。

- [ ] **Step 4: index.ts 注入 onInsertUploading 给 SlashCommand**

在 `index.ts` 的 `SlashCommand.configure({...})`（约第 88-90 行）改为：

```typescript
        SlashCommand.configure({
          onImageUpload: this.options.onImageUpload,
          onInsertUploading: this.coordinator
            ? (file) => this.coordinator!.insertUploading(file)
            : undefined,
        }),
```

- [ ] **Step 5: 构建验证**

Run: `make build-editor-incremental`
Expected: `✓ built in ...`

- [ ] **Step 6: Commit**

```bash
git add libs/tiptap-editor/src/slash-command.ts libs/tiptap-editor/src/index.ts
git commit -m "feat(editor): route slash-command image upload through coordinator"
```

---

## Task 7: NodeView 样式（upload-image overlay/spinner/error card）

**Files:**
- Modify: `libs/tiptap-editor/src/style.css`

在现有 Image 样式块（第 361-373 行）之后追加 NodeView 三种态样式。

- [ ] **Step 1: 追加亮色样式**

在 `style.css` 第 373 行（`.ProseMirror-selectednode` 块之后）追加：

```css
/* 上传图片占位符 NodeView */
.upload-image-container {
  position: relative;
  display: inline-block;
  max-width: 100%;
}
.upload-image-container img {
  max-width: 100%;
  height: auto;
  border-radius: 6px;
  margin: 1em 0;
  display: block;
}
.upload-image-container.is-error img {
  opacity: 0.5;
  filter: grayscale(0.5);
}
.upload-image-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border-radius: 6px;
  pointer-events: none;
}
.upload-image-container.is-error .upload-image-overlay {
  background: rgba(0, 0, 0, 0.35);
  pointer-events: auto;
}
.upload-spinner {
  width: 28px;
  height: 28px;
  border: 3px solid rgba(255, 255, 255, 0.4);
  border-top-color: #fff;
  border-radius: 50%;
  animation: upload-spin 0.8s linear infinite;
}
@keyframes upload-spin {
  to { transform: rotate(360deg); }
}
.upload-overlay-text {
  color: #fff;
  font-size: 13px;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
}
.upload-error-icon {
  color: #fff;
  font-size: 22px;
  line-height: 1;
}
.upload-error-msg {
  color: #fff;
  font-size: 13px;
  text-align: center;
  padding: 0 12px;
  max-width: 90%;
  word-break: break-word;
}
.upload-error-actions {
  display: flex;
  gap: 8px;
}
.upload-btn {
  padding: 3px 12px;
  font-size: 12px;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.6);
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
  cursor: pointer;
}
.upload-btn:hover {
  background: rgba(255, 255, 255, 0.3);
}
.upload-btn-remove {
  border-color: rgba(254, 202, 202, 0.7);
  color: #fecaca;
}
.upload-btn-remove:hover {
  background: rgba(239, 68, 68, 0.3);
}
```

- [ ] **Step 2: 追加暗色样式**

在 `style.css` 暗色 Image 块（约第 419-422 行 `.dark ... img.ProseMirror-selectednode`）之后追加：

```css
.dark .upload-image-container.is-error img {
  opacity: 0.45;
}
.dark .upload-btn {
  border-color: rgba(200, 200, 200, 0.4);
}
.dark .upload-btn-remove {
  border-color: rgba(239, 68, 68, 0.5);
}
```

- [ ] **Step 3: 构建验证**

Run: `make build-editor-incremental`
Expected: `✓ built in ...`，`public/tiptap/editor.css` 包含新选择器。

- [ ] **Step 4: Commit**

```bash
git add libs/tiptap-editor/src/style.css
git commit -m "style(editor): add upload placeholder NodeView styles (overlay/spinner/error)"
```

---

## Task 8: write.rs — 新增 signal 与类型定义

**Files:**
- Modify: `src/pages/admin/write.rs`

加入 `UploadsInFlight` 类型、`uploads_in_flight` 与 `upload_errors` signal。为后续轮询和渲染做准备。

- [ ] **Step 1: 添加类型定义**

在 `write.rs` 的 `use` 语句之后、`Write()` 组件之前（约第 20 行附近，`use crate::router::Route;` 之后）添加：

```rust
/// 当前编辑器内进行中的上传计数（来自轮询 counts）。
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
```

- [ ] **Step 2: 添加 signal 声明**

在 `write_editor` 的 signal 声明块（第 69 行 `let mut edit_post = use_signal(|| None::<Post>);` 之后）添加：

```rust
    // 上传状态：当前进行中计数 + 顶部失败提示堆叠
    let mut uploads_in_flight = use_signal(UploadsInFlight::default);
    let mut upload_errors: Signal<Vec<UploadErrorEntry>> = use_signal(Vec::new);
```

- [ ] **Step 3: 编译验证**

Run: `cargo check 2>&1 | tail -10`
Expected: 无错误（signal 类型正确，可能有 `unused` 警告，后续任务会消费）。

- [ ] **Step 4: Commit**

```bash
git add src/pages/admin/write.rs
git commit -m "feat(write): add upload state signals for placeholder tracking"
```

---

## Task 9: write.rs — 轮询 effect 消费 window.__tiptap_uploads

**Files:**
- Modify: `src/pages/admin/write.rs`

新增 `use_future` 500ms 轮询 `window.__tiptap_uploads`，消费 events 更新 signal。

- [ ] **Step 1: 添加 use_future 轮询**

在 ready-polling `use_effect`（第 201-244 行）之后添加新的 `use_future`：

```rust
    // 轮询 window.__tiptap_uploads，消费上传事件并更新 signal
    use_future(move || {
        let mut uploads_in_flight = uploads_in_flight;
        let mut upload_errors = upload_errors;
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                use std::collections::HashSet;
                let mut seen_error_ids: HashSet<String> = HashSet::new();
                loop {
                    // 500ms 间隔
                    if let Ok(promise_val) = js_sys::eval("new Promise(r => setTimeout(r, 500))") {
                        if let Ok(promise) = promise_val.dyn_into::<js_sys::Promise>() {
                            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                        }
                    }

                    // 读取并清空 events，读取 counts
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
                        #[derive(serde::Deserialize)]
                        struct SnapshotEvent {
                            kind: String,
                            #[serde(rename = "uploadId")]
                            upload_id: String,
                            #[serde(rename = "fileName")]
                            file_name: String,
                            #[serde(rename = "errorMsg")]
                            error_msg: Option<String>,
                        }
                        #[derive(serde::Deserialize)]
                        struct Snapshot {
                            events: Vec<SnapshotEvent>,
                            counts: Counts,
                        }
                        #[derive(serde::Deserialize, Default, Clone, Copy)]
                        struct Counts {
                            uploading: u32,
                            error: u32,
                        }
                        if let Ok(parsed) = serde_json::from_str::<Snapshot>(&json) {
                            for ev in parsed.events {
                                match ev.kind.as_str() {
                                    "error" => {
                                        if !seen_error_ids.contains(&ev.upload_id) {
                                            seen_error_ids.insert(ev.upload_id.clone());
                                            upload_errors.write().push(UploadErrorEntry {
                                                id: ev.upload_id,
                                                file_name: ev.file_name,
                                                message: ev.error_msg.unwrap_or_else(|| "上传失败".to_string()),
                                            });
                                        }
                                    }
                                    "success" | "removed" => {
                                        seen_error_ids.remove(&ev.upload_id);
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
        }
    });
```

注：闭包开头显式 capture `uploads_in_flight` 和 `upload_errors` 两个 mutable signal（Dioxus 的 use_future 闭包需捕获要写的 signal）。`serde::Deserialize` 派生需要 `serde` 在依赖中——项目已用 serde，确认 `Cargo.toml` 的 `[dependencies] serde = { version = "...", features = ["derive"] }`。若 features 无 derive，用 `serde_json::Value` 手动解析代替派生。

- [ ] **Step 2: 确认 serde derive feature 可用**

Run: `grep -A2 '^serde' Cargo.toml | head -5`
Expected: 看到 `features = ["derive"]` 或类似。若无 derive，将 Step 1 的派生改为 `serde_json::Value` 手动解析（`v["events"][i]["kind"].as_str()` 等）。

- [ ] **Step 3: 编译验证**

Run: `cargo check 2>&1 | tail -15`
Expected: 无错误。若 serde derive 缺失，改用 Value 手动解析后重试。

- [ ] **Step 4: Commit**

```bash
git add src/pages/admin/write.rs
git commit -m "feat(write): poll window.__tiptap_uploads for upload events and counts"
```

---

## Task 10: write.rs — 顶部上传失败提示渲染

**Files:**
- Modify: `src/pages/admin/write.rs`

在现有 error/success 提示块（第 452-469 行）之后渲染 upload_errors，每条带 ×关闭（同时删占位符）。

- [ ] **Step 1: 添加顶部提示渲染**

在 `success` 提示 div（第 465-469 行）之后、底部操作栏（第 471 行）之前添加：

```rust
            // 上传失败提示：多条堆叠，×关闭同时删除编辑器内失败占位符
            for err in upload_errors.read().iter() {
                div { class: "flex-shrink-0 flex items-center justify-between gap-3 px-4 py-2 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded-xl text-sm border border-red-100 dark:border-red-900/30 mb-2",
                    span { "图片上传失败: {err.file_name} — {err.message}" }
                    button {
                        class: "shrink-0 text-red-400 hover:text-red-600 cursor-pointer text-lg leading-none",
                        aria_label: "关闭提示",
                        onclick: move |_| {
                            // 关闭提示同时删除编辑器内失败占位符（避免孤儿）
                            let id = err.id.clone();
                            let _ = js_sys::eval(&format!(
                                "(function(){{var e=window.TiptapEditor&&window.TiptapEditor._instances&&window.TiptapEditor._instances.get('tiptap-editor');if(e&&e.removeUploadByUploadId){{e.removeUploadByUploadId({:?});}}}})()",
                                id
                            ));
                            upload_errors.write().retain(|e| e.id != id);
                        },
                        "×"
                    }
                }
            }
```

- [ ] **Step 2: 编译验证**

Run: `cargo check 2>&1 | tail -10`
Expected: 无错误。

- [ ] **Step 3: Commit**

```bash
git add src/pages/admin/write.rs
git commit -m "feat(write): render stacked upload failure notices with dismiss"
```

---

## Task 11: write.rs — 保存拦截（counts 检查 + markdown 兜底）

**Files:**
- Modify: `src/pages/admin/write.rs`

在 `on_submit` 开头加 counts 检查；在拿到 markdown 后加 blob: 兜底扫描。

- [ ] **Step 1: 在 on_submit 开头加 counts 检查**

将 `on_submit` 开头（第 248 行 `let on_submit = move |_| {` 之后，title 校验之前）改为：

```rust
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
```

- [ ] **Step 2: 在拿到 markdown 后加 blob: 兜底扫描**

将 markdown 读取与空内容检查（第 258-268 行）改为：

```rust
            let md = js_sys::eval(r#"
                (function() {
                    var editor = window.TiptapEditor && window.TiptapEditor._instances && window.TiptapEditor._instances.get('tiptap-editor');
                    return editor ? editor.getMarkdown() : (window.__tiptap_content || '');
                })()
            "#).ok().and_then(|v| v.as_string()).unwrap_or_default();

            // 兜底：扫描残留的 blob: 或 data-upload-state（轮询窗口期漏判防护）
            if md.contains("blob:") || md.contains("data-upload-state") {
                error.set(Some("检测到未完成上传的图片，请处理后保存".to_string()));
                return;
            }

            if md.trim().is_empty() {
                error.set(Some("内容不能为空".to_string()));
                return;
            }
```

- [ ] **Step 3: 编译验证**

Run: `cargo check 2>&1 | tail -10`
Expected: 无错误。

- [ ] **Step 4: Commit**

```bash
git add src/pages/admin/write.rs
git commit -m "feat(write): block save when uploads in flight or blob residue detected"
```

---

## Task 12: write.rs — fetch 改造透传服务端中文错误

**Files:**
- Modify: `src/pages/admin/write.rs`

当前 `onImageUpload` 的 fetch 在非 2xx 时丢弃服务端错误体。改为读取 `data.error`。

- [ ] **Step 1: 改写 fetch 的非 2xx 处理**

将 eval 字符串中 `onImageUpload` 的 `.then(function(response) {...})`（第 169-173 行）改为：

```javascript
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
```

- [ ] **Step 2: 构建验证**

Run: `make build-editor-incremental && dx check 2>&1 | tail -5`
Expected: tiptap 构建成功，dx check 无问题。

- [ ] **Step 3: Commit**

```bash
git add src/pages/admin/write.rs
git commit -m "fix(write): surface server error messages in upload failures"
```

---

## Task 13: 全量验证

**Files:** 无新改动，仅验证。

- [ ] **Step 1: cargo test + clippy**

Run: `cargo test 2>&1 | tail -5 && cargo clippy --all-targets 2>&1 | tail -5`
Expected: 测试全过，clippy 无警告。

- [ ] **Step 2: dx check**

Run: `dx check 2>&1 | tail -3`
Expected: `No issues found.`

- [ ] **Step 3: tiptap 构建产物校验**

Run: `make build-editor-incremental 2>&1 | tail -5`
Expected: `✓ built in ...`

- [ ] **Step 4: 手动验证清单（需 dev server 运行）**

启动 `make dev`，浏览器打开编辑器页面，硬刷新（Cmd+Shift+R）后逐项验证：

- [ ] 粘贴图片：编辑器立即显示本地预览 + "上传中…"遮罩
- [ ] 上传成功：遮罩消失，src 换为服务端 URL，无光标跳动
- [ ] 拖拽图片：同上，且插入到拖放位置
- [ ] 斜杠 /上传图片：弹出文件选择器，选图后出现占位符
- [ ] 上传超大文件（>5MB）：占位符变红，显示"文件超过大小限制"
- [ ] 失败占位符上点"重试"：转回 uploading，重跑上传
- [ ] 失败占位符上点"移除"：节点删除
- [ ] 上传失败时顶部出现堆叠提示（文件名 + 错误原因）
- [ ] 多张同时失败：顶部多条堆叠
- [ ] 编辑器内移除失败占位符：顶部对应提示同步消失
- [ ] 点顶部 ×：编辑器内对应占位符同步删除
- [ ] 有 uploading 占位符时点保存：被阻止，提示"有 N 张图片正在上传"
- [ ] 有 error 占位符时点保存：被阻止，提示"有 N 张图片上传失败"
- [ ] 全部完成后保存：正常通过

- [ ] **Step 5: 最终 commit（如有验证中发现的小修复）**

```bash
# 仅当验证中修复了问题时
git add -A
git commit -m "fix(editor): address verification findings"
```

---

## 完成标准

- 所有 Task 的 Step checkbox 打勾
- `cargo test` / `cargo clippy` / `dx check` / `make build-editor-incremental` 全部通过
- 手动验证清单全部通过
- spec（`docs/superpowers/specs/2026-06-22-upload-placeholder-design.md`）的 11 条验收标准全部满足

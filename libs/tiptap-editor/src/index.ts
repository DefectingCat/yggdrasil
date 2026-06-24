import { Editor } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { Markdown } from '@tiptap/markdown'
import { TableKit } from '@tiptap/extension-table'
import { TaskList, TaskItem } from '@tiptap/extension-list'
import { FileHandler } from '@tiptap/extension-file-handler'
import { SlashCommand } from './slash-command'
import { UploadCoordinator, UPLOAD_COORDINATOR_STORAGE_KEY, type UploadEvent } from './upload-coordinator'
import { UploadImage } from './upload-image'
import './style.css'

/**
 * 编辑器选项。用 class 而非 interface，使其编译后保留运行时构造函数——
 * Rust 侧 wasm-bindgen 的 `#[wasm_bindgen(constructor)]` 会生成 `new EditorOptions()`，
 * 需要一个全局可访问的构造函数（interface 会被 TS 擦除）。
 * 字段全部可选，Rust 通过 setter（set_placeholder/set_on_update/...）逐个赋值。
 *
 * 不用 export（避免 IIFE 的 named export 与 default 冲突），而是挂到 globalThis，
 * 让 wasm-bindgen glue 里的裸标识符 `EditorOptions` 能解析到。
 */
class EditorOptions {
  content?: string
  placeholder?: string
  onUpdate?: (markdown: string) => void
  onFocus?: () => void
  onBlur?: () => void
  editable?: boolean
  // 图片上传回调
  onImageUpload?: (file: File) => Promise<string>
  // 编辑器实例创建完成（含 coordinator）后同步触发一次，替代 window.__tiptap_ready 轮询
  onReady?: () => void
  // 上传状态事件（error/success/removed + counts），替代 window.__tiptap_uploads 轮询
  onUploadEvent?: (event: UploadEvent) => void
}

// wasm-bindgen 生成的 glue 用裸标识符 `new EditorOptions()` 从全局解析，
// IIFE 的 name 只能挂一个全局（TiptapEditor），这里手动把 EditorOptions 也挂到 window 上。
;(window as unknown as Record<string, unknown>).EditorOptions = EditorOptions

class TiptapEditorInstance {
  private editor: Editor | null = null
  private container: HTMLElement
  private options: EditorOptions

  private isSourceMode = false
  private sourceTextarea: HTMLTextAreaElement | null = null
  private toggleButton: HTMLButtonElement | null = null
  private coordinator: UploadCoordinator | null = null

  constructor(container: HTMLElement, options: EditorOptions = new EditorOptions()) {
    this.container = container
    this.options = options
    this.init()
  }

  private init() {
    const el = document.createElement('div')
    el.className = 'tiptap-editor'
    this.container.appendChild(el)

    // 源码模式切换按钮：悬浮于编辑器右上角
    this.toggleButton = document.createElement('button')
    this.toggleButton.className = 'tiptap-toggle-btn'
    this.toggleButton.type = 'button'
    this.toggleButton.title = '切换 Markdown 源码'
    this.toggleButton.textContent = '</>'
    this.toggleButton.addEventListener('click', () => this.toggleSource())
    el.appendChild(this.toggleButton)

    // 源码模式 textarea：初始隐藏，与 ProseMirror 共用同一区域
    this.sourceTextarea = document.createElement('textarea')
    this.sourceTextarea.className = 'tiptap-source-textarea'
    this.sourceTextarea.hidden = true
    this.sourceTextarea.placeholder = '在此输入 Markdown 源码...'
    this.sourceTextarea.spellcheck = false
    this.sourceTextarea.addEventListener('input', () => {
      // 源码模式下通过 onUpdate 回调同步内容（替代旧版 window.__tiptap_content 缓存）
      if (this.options.onUpdate) {
        this.options.onUpdate(this.sourceTextarea!.value)
      }
    })
    el.appendChild(this.sourceTextarea)

    this.editor = new Editor({
      element: el,
      extensions: [
        StarterKit.configure({
          heading: {
            levels: [1, 2, 3],
          },
          link: {
            openOnClick: false,
            autolink: true,
            linkOnPaste: true,
            HTMLAttributes: { rel: 'noopener noreferrer', target: '_blank' },
          },
        }),
        Markdown,
        TableKit,
        UploadImage,
        TaskList,
        TaskItem.configure({ nested: true }),
        // 把宿主注入的图片上传回调透传给斜杠命令扩展，使 /上传图片 命令可用。
        // 注意：闭包延迟读取 this.coordinator（它在 editor 创建后才实例化）。
        SlashCommand.configure({
          onImageUpload: this.options.onImageUpload,
          onInsertUploading: this.options.onImageUpload
            ? (file) => this.coordinator?.insertUploading(file)
            : undefined,
        }),
        FileHandler.configure({
          allowedMimeTypes: ['image/jpeg', 'image/png', 'image/gif', 'image/webp'],
          onPaste: (_editor, files) => {
            if (this.coordinator) {
              files.forEach((file) => this.coordinator!.insertUploading(file))
            }
          },
          onDrop: (_editor, files, pos) => {
            if (this.coordinator) {
              files.forEach((file) => this.coordinator!.insertUploading(file, pos))
            }
          },
        }),
      ],
      content: this.options.content || '',
      editable: this.options.editable !== false,
      autofocus: false,
      onUpdate: ({ editor }) => {
        if (this.options.onUpdate) {
          this.options.onUpdate(editor.getMarkdown())
        }
      },
      onFocus: () => {
        if (this.options.onFocus) {
          this.options.onFocus()
        }
      },
      onBlur: () => {
        if (this.options.onBlur) {
          this.options.onBlur()
        }
      },
    })

    // 创建上传协调器，挂到 editor.storage 供 NodeView 按实例读取（支持多编辑器实例）。
    // onUploadEvent 透传给 coordinator.emit，未提供时空操作兜底。
    if (this.options.onImageUpload) {
      this.coordinator = new UploadCoordinator(
        this.editor,
        this.options.onImageUpload,
        this.options.onUploadEvent ?? (() => {}),
      )
      // editor.storage 是开放式索引签名；自定义 key 需绕过严格检查（Tiptap 官方扩展 storage 模式）。
      ;(this.editor.storage as unknown as Record<string, unknown>)[UPLOAD_COORDINATOR_STORAGE_KEY] = this.coordinator
    }

    // 通知宿主编辑器已就绪（替代 window.__tiptap_ready 轮询）
    this.options.onReady?.()
  }

  getMarkdown(): string {
    // 源码模式下直接返回 textarea 内容，确保提交逻辑无需感知视图模式
    if (this.isSourceMode && this.sourceTextarea) {
      return this.sourceTextarea.value
    }
    return this.editor?.getMarkdown() || ''
  }

  /**
   * 在富文本模式与 Markdown 源码模式之间切换。
   * 切换时同步内容：富文本 → 源码用 getMarkdown 导出；源码 → 富文本用 setMarkdown 回填。
   */
  toggleSource(): void {
    if (!this.editor || !this.sourceTextarea || !this.toggleButton) return

    // ProseMirror 的实际 DOM 节点（.ProseMirror），用于切换显隐。
    const proseMirrorDom = this.editor.view.dom
    if (!this.isSourceMode) {
      // 富文本 → 源码：导出当前 Markdown 到 textarea
      this.sourceTextarea.value = this.editor.getMarkdown()
      // 必须在 display:'none' 之前读取滚动比例——隐藏后 scrollTop 会被浏览器归零
      const pmRatio = this.getScrollRatio(proseMirrorDom)
      proseMirrorDom.style.display = 'none'
      this.sourceTextarea.hidden = false
      this.applyScrollRatio(this.sourceTextarea, pmRatio)
      // focus() 会触发浏览器把光标所在行滚动进可视区域，光标默认在内容末尾，
      // 会把 textarea 拉到底部覆盖上面设好的滚动位置。因此先记录、focus 后再恢复。
      const scrollTopBeforeFocus = this.sourceTextarea.scrollTop
      this.sourceTextarea.focus()
      this.sourceTextarea.scrollTop = scrollTopBeforeFocus
      this.toggleButton.textContent = '✎'
      this.toggleButton.title = '切换富文本'
      this.isSourceMode = true
    } else {
      // 源码 → 富文本：把 textarea 内容回填到编辑器
      const md = this.sourceTextarea.value
      // 先记录源码视图的滚动比例（setContent 会重建文档，必须在替换前拿到比例）
      const sourceRatio = this.getScrollRatio(this.sourceTextarea)
      this.setMarkdown(md)
      this.sourceTextarea.hidden = true
      proseMirrorDom.style.display = ''
      // 等待 DOM 布局更新后，按比例同步富文本视图滚动位置
      requestAnimationFrame(() => {
        this.applyScrollRatio(proseMirrorDom, sourceRatio)
      })
      this.toggleButton.textContent = '</>'
      this.toggleButton.title = '切换 Markdown 源码'
      this.isSourceMode = false
      // 注意：不调用 editor.commands.focus()，它会强制滚动到光标位置（默认文档末尾），破坏比例同步
    }
  }

  /**
   * 读取可滚动容器的滚动比例（0~1）。
   * 比例 = scrollTop / 可滚动总距离，避免两种模式内容高度不同导致像素无法直接对应。
   */
  private getScrollRatio(el: HTMLElement): number {
    const max = el.scrollHeight - el.clientHeight
    if (max <= 0) return 0
    return el.scrollTop / max
  }

  /**
   * 按比例设置目标容器的滚动位置。
   */
  private applyScrollRatio(el: HTMLElement, ratio: number): void {
    const max = el.scrollHeight - el.clientHeight
    if (max <= 0) return
    el.scrollTop = max * ratio
  }

  setMarkdown(content: string): void {
    this.editor?.commands.setContent(content, { emitUpdate: false, contentType: 'markdown' })
  }

  getHTML(): string {
    return this.editor?.getHTML() || ''
  }

  focus(): void {
    this.editor?.commands.focus()
  }

  blur(): void {
    this.editor?.commands.blur()
  }

  isEmpty(): boolean {
    return this.editor?.isEmpty ?? true
  }

  /** Rust 侧"×关闭"提示时调用（通过 eval）。返回是否成功删除。 */
  removeUploadByUploadId(uploadId: string): boolean {
    return this.coordinator?.removeUpload(uploadId) ?? false
  }

  destroy(): void {
    this.editor?.destroy()
    this.editor = null
    // coordinator 通过 editor.storage 访问，随 editor 实例一同回收，无需显式清除引用。
    this.coordinator = null
    // 清理源码模式相关引用（容器 innerHTML 已清空，DOM 会随之移除）
    this.sourceTextarea = null
    this.toggleButton = null
    this.isSourceMode = false
    this.container.innerHTML = ''
  }
}

const TiptapEditor = {
  _instances: new Map<string, TiptapEditorInstance>(),

  create(containerId: string, options: EditorOptions = new EditorOptions()): TiptapEditorInstance | null {
    const container = document.getElementById(containerId)
    if (!container) {
      console.error(`[TiptapEditor] Container not found: #${containerId}`)
      return null
    }

    const existing = this._instances.get(containerId)
    if (existing) {
      existing.destroy()
    }

    const instance = new TiptapEditorInstance(container, options)
    this._instances.set(containerId, instance)
    return instance
  },
}

export default TiptapEditor

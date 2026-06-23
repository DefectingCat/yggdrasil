import { Editor } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { Markdown } from '@tiptap/markdown'
import { TableKit } from '@tiptap/extension-table'
import { TaskList, TaskItem } from '@tiptap/extension-list'
import { FileHandler } from '@tiptap/extension-file-handler'
import { SlashCommand } from './slash-command'
import { UploadCoordinator, type UploadEvent } from './upload-coordinator'
import { UploadImage, setUploadCoordinator } from './upload-image'
import './style.css'

export interface EditorOptions {
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

class TiptapEditorInstance {
  private editor: Editor | null = null
  private container: HTMLElement
  private options: EditorOptions
  // 源码模式相关状态
  private isSourceMode = false
  private sourceTextarea: HTMLTextAreaElement | null = null
  private toggleButton: HTMLButtonElement | null = null
  private coordinator: UploadCoordinator | null = null

  constructor(container: HTMLElement, options: EditorOptions = {}) {
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
      // 保持全局缓存与 onUpdate 回调一致
      window.__tiptap_content = this.sourceTextarea!.value
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
        Markdown.configure({
          html: false,
        }),
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

    // 创建上传协调器，注入给 NodeView 的 onRetry/onRemove
    // onUploadEvent 透传给 coordinator.emit，未提供时空操作兜底
    if (this.options.onImageUpload) {
      this.coordinator = new UploadCoordinator(
        this.editor,
        this.options.onImageUpload,
        this.options.onUploadEvent ?? (() => {}),
      )
      setUploadCoordinator(this.coordinator)
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
    this.coordinator = null
    // 清除 NodeView 的 coordinator 引用，避免指向已销毁的实例
    setUploadCoordinator(null)
    // 清理源码模式相关引用（容器 innerHTML 已清空，DOM 会随之移除）
    this.sourceTextarea = null
    this.toggleButton = null
    this.isSourceMode = false
    this.container.innerHTML = ''
  }
}

const TiptapEditor = {
  _instances: new Map<string, TiptapEditorInstance>(),

  create(containerId: string, options: EditorOptions = {}): TiptapEditorInstance | null {
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

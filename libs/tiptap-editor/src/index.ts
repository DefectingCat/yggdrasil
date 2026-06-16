import { Editor } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { Markdown } from '@tiptap/markdown'
import { TableKit } from '@tiptap/extension-table'
import { Image } from '@tiptap/extension-image'
import { TaskList, TaskItem } from '@tiptap/extension-list'
import { FileHandler } from '@tiptap/extension-file-handler'
import { SlashCommand } from './slash-command'
import './style.css'

export interface EditorOptions {
  content?: string
  placeholder?: string
  onUpdate?: (markdown: string) => void
  onFocus?: () => void
  onBlur?: () => void
  editable?: boolean
  // 新增：图片上传回调
  onImageUpload?: (file: File) => Promise<string>
}

class TiptapEditorInstance {
  private editor: Editor | null = null
  private container: HTMLElement
  private options: EditorOptions
  // 源码模式相关状态
  private isSourceMode = false
  private sourceTextarea: HTMLTextAreaElement | null = null
  private toggleButton: HTMLButtonElement | null = null

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
        Image.configure({ allowBase64: true }),
        TaskList,
        TaskItem.configure({ nested: true }),
        SlashCommand,
        FileHandler.configure({
          allowedMimeTypes: ['image/jpeg', 'image/png', 'image/gif', 'image/webp'],
          onPaste: (editor, files) => {
            if (this.options.onImageUpload) {
              files.forEach((file) => {
                this.options.onImageUpload!(file)
                  .then((url) => {
                    editor.chain().focus().setImage({ src: url }).run()
                  })
                  .catch((err) => {
                    const msg = err instanceof Error ? err.message : String(err)
                    console.error('[TiptapEditor] Upload failed:', msg)
                  })
              })
            }
          },
          onDrop: (editor, files, pos) => {
            if (this.options.onImageUpload) {
              files.forEach((file) => {
                this.options.onImageUpload!(file)
                  .then((url) => {
                    editor.chain().focus().insertContentAt(pos, {
                      type: 'image',
                      attrs: { src: url }
                    }).run()
                  })
                  .catch((err) => {
                    const msg = err instanceof Error ? err.message : String(err)
                    console.error('[TiptapEditor] Upload failed:', msg)
                  })
              })
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
      proseMirrorDom.style.display = 'none'
      this.sourceTextarea.hidden = false
      this.sourceTextarea.focus()
      this.toggleButton.textContent = '✎'
      this.toggleButton.title = '切换富文本'
      this.isSourceMode = true
    } else {
      // 源码 → 富文本：把 textarea 内容回填到编辑器
      const md = this.sourceTextarea.value
      this.setMarkdown(md)
      this.sourceTextarea.hidden = true
      proseMirrorDom.style.display = ''
      this.toggleButton.textContent = '</>'
      this.toggleButton.title = '切换 Markdown 源码'
      this.isSourceMode = false
      this.editor.commands.focus()
    }
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

  destroy(): void {
    this.editor?.destroy()
    this.editor = null
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

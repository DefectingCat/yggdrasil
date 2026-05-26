import { Editor } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { Markdown } from '@tiptap/markdown'
import './style.css'

export interface EditorOptions {
  content?: string
  placeholder?: string
  onUpdate?: (markdown: string) => void
  onFocus?: () => void
  onBlur?: () => void
  editable?: boolean
}

class TiptapEditorInstance {
  private editor: Editor | null = null
  private container: HTMLElement
  private options: EditorOptions

  constructor(container: HTMLElement, options: EditorOptions = {}) {
    this.container = container
    this.options = options
    this.init()
  }

  private init() {
    const el = document.createElement('div')
    el.className = 'tiptap-editor'
    this.container.appendChild(el)

    this.editor = new Editor({
      element: el,
      extensions: [
        StarterKit.configure({
          heading: {
            levels: [1, 2, 3],
          },
        }),
        Markdown.configure({
          html: false,
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
    return this.editor?.getMarkdown() || ''
  }

  setMarkdown(content: string): void {
    this.editor?.commands.setContent(content, false, { contentType: 'markdown' })
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

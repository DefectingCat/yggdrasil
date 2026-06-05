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
    return this.editor?.getMarkdown() || ''
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

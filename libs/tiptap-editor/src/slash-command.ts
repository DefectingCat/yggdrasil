import { Extension, type Range } from '@tiptap/core'
import { Suggestion, type SuggestionProps, type SuggestionKeyDownProps } from '@tiptap/suggestion'
import { PluginKey } from '@tiptap/pm/state'

interface CommandItem {
  title: string
  description: string
  icon: string
  command: (props: { editor: any; range: Range }) => void
}

const COMMANDS: CommandItem[] = [
  {
    title: '标题 1',
    description: '大标题',
    icon: 'H1',
    command: ({ editor, range }) => {
      editor.chain().focus().deleteRange(range).setHeading({ level: 1 }).run()
    },
  },
  {
    title: '标题 2',
    description: '中标题',
    icon: 'H2',
    command: ({ editor, range }) => {
      editor.chain().focus().deleteRange(range).setHeading({ level: 2 }).run()
    },
  },
  {
    title: '标题 3',
    description: '小标题',
    icon: 'H3',
    command: ({ editor, range }) => {
      editor.chain().focus().deleteRange(range).setHeading({ level: 3 }).run()
    },
  },
  {
    title: '无序列表',
    description: '创建无序列表',
    icon: '•',
    command: ({ editor, range }) => {
      editor.chain().focus().deleteRange(range).toggleBulletList().run()
    },
  },
  {
    title: '有序列表',
    description: '创建有序列表',
    icon: '1.',
    command: ({ editor, range }) => {
      editor.chain().focus().deleteRange(range).toggleOrderedList().run()
    },
  },
  {
    title: '任务列表',
    description: '创建任务列表',
    icon: '☑',
    command: ({ editor, range }) => {
      editor.chain().focus().deleteRange(range).toggleTaskList().run()
    },
  },
  {
    title: '引用',
    description: '插入引用块',
    icon: '❝',
    command: ({ editor, range }) => {
      editor.chain().focus().deleteRange(range).toggleBlockquote().run()
    },
  },
  {
    title: '代码块',
    description: '插入代码块',
    icon: '<>',
    command: ({ editor, range }) => {
      editor.chain().focus().deleteRange(range).toggleCodeBlock().run()
    },
  },
  {
    title: '分割线',
    description: '插入水平分割线',
    icon: '—',
    command: ({ editor, range }) => {
      editor.chain().focus().deleteRange(range).setHorizontalRule().run()
    },
  },
  {
    title: '表格',
    description: '插入 3×3 表格',
    icon: '▦',
    command: ({ editor, range }) => {
      editor.chain().focus().deleteRange(range).insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run()
    },
  },
  {
    title: '图片',
    description: '插入图片',
    icon: '🖼',
    command: ({ editor, range }) => {
      const url = window.prompt('输入图片 URL')
      if (url) {
        editor.chain().focus().deleteRange(range).setImage({ src: url }).run()
      }
    },
  },
  {
    title: '链接',
    description: '插入链接',
    icon: '🔗',
    command: ({ editor, range }) => {
      const url = window.prompt('输入链接 URL')
      if (url) {
        editor.chain().focus().deleteRange(range).setLink({ href: url }).insertContent(url).run()
      }
    },
  },
]

const SlashCommandPluginKey = new PluginKey('slashCommand')

function createPopup(props: SuggestionProps<CommandItem>) {
  const component = document.createElement('div')
  component.classList.add('slash-command')

  const list = document.createElement('div')
  list.classList.add('slash-command-list')
  component.appendChild(list)

  let selectedIndex = 0
  let currentItems: CommandItem[] = []

  function renderItems(items: CommandItem[]) {
    currentItems = items
    list.innerHTML = ''
    selectedIndex = 0

    items.forEach((item, index) => {
      const el = document.createElement('div')
      el.classList.add('slash-command-item')
      if (index === 0) el.classList.add('is-selected')

      el.innerHTML = `
        <div class="slash-command-item-icon">${item.icon}</div>
        <div class="slash-command-item-text">
          <div class="slash-command-item-title">${item.title}</div>
          <div class="slash-command-item-desc">${item.description}</div>
        </div>
      `

      el.addEventListener('click', () => {
        props.command(item)
      })

      el.addEventListener('mouseenter', () => {
        selectedIndex = index
        updateSelection()
      })

      list.appendChild(el)
    })
  }

  function updateSelection() {
    const children = list.children
    for (let i = 0; i < children.length; i++) {
      if (i === selectedIndex) {
        children[i].classList.add('is-selected')
      } else {
        children[i].classList.remove('is-selected')
      }
    }
    children[selectedIndex]?.scrollIntoView({ block: 'nearest' })
  }

  function selectItem() {
    if (currentItems[selectedIndex]) {
      props.command(currentItems[selectedIndex])
    }
  }

  function updatePosition() {
    const rect = props.clientRect?.()
    if (!rect) return
    component.style.left = `${rect.left}px`
    component.style.top = `${rect.bottom + 4}px`
  }

  renderItems(props.items)
  document.body.appendChild(component)
  updatePosition()

  return {
    component,
    updateItems(items: CommandItem[]) {
      renderItems(items)
    },
    updatePosition,
    onKeyDown({ event }: SuggestionKeyDownProps): boolean {
      if (event.key === 'ArrowUp') {
        event.preventDefault()
        selectedIndex = (selectedIndex - 1 + currentItems.length) % currentItems.length
        updateSelection()
        return true
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault()
        selectedIndex = (selectedIndex + 1) % currentItems.length
        updateSelection()
        return true
      }
      if (event.key === 'Enter') {
        event.preventDefault()
        selectItem()
        return true
      }
      if (event.key === 'Escape') {
        event.preventDefault()
        return true
      }
      return false
    },
    destroy() {
      component.remove()
    },
  }
}

export const SlashCommand = Extension.create({
  name: 'slashCommand',

  addProseMirrorPlugins() {
    return [
      Suggestion<CommandItem>({
        pluginKey: SlashCommandPluginKey,
        editor: this.editor,
        char: '/',
        items: ({ query }) => {
          return COMMANDS.filter(
            (item) =>
              item.title.toLowerCase().includes(query.toLowerCase()) ||
              item.description.toLowerCase().includes(query.toLowerCase())
          )
        },
        render() {
          let popup: ReturnType<typeof createPopup> | null = null

          return {
            onStart(props) {
              popup = createPopup(props)
            },
            onUpdate(props) {
              if (!popup) return
              popup.updateItems(props.items)
              popup.updatePosition()
            },
            onKeyDown(props) {
              if (!popup) return false
              return popup.onKeyDown(props)
            },
            onExit() {
              if (popup) {
                popup.destroy()
                popup = null
              }
            },
          }
        },
        command: ({ editor, range, props: item }) => {
          item.command({ editor, range })
        },
      }),
    ]
  },
})

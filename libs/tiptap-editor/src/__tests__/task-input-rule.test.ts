// @vitest-environment happy-dom
import { describe, it, expect, beforeEach } from 'vitest'
import { Editor } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { TaskList, TaskItem } from '@tiptap/extension-list'
import { TaskInputRule } from '../task-input-rule'

/**
 * TaskInputRule 单元测试（happy-dom 真实 DOM + 真实 Editor）。
 *
 * input rule 在真实输入时由 ProseMirror 的 handleTextInput 触发。测试里用
 * `insertContentAt(pos, text, { applyInputRules: true })` 模拟——它走与真实输入
 * 相同的 inputRulesPlugin（经 applyInputRules meta，setTimeout 异步触发），
 * 因此断言前需 await 一个宏任务让 setTimeout 排空。
 *
 * 46 个回归测试见 upload-coordinator/upload-image/slash-command，这里只覆盖
 * TaskInputRule 自身的触发/checked/不误判。
 */

/** 等待 applyInputRules 的 setTimeout 排空。 */
function flushInputRules() {
  return new Promise((resolve) => setTimeout(resolve, 0))
}

/**
 * 取 JSON 文档首个节点(任务列表/无序列表等)。
 * getJSON() 的 content 项是 Node | Text 联合,Text 无 content/attrs,
 * 断言为 any 窄化(本测试只构造块节点场景)。
 */
function firstBlock(editor: Editor): any {
  return editor.getJSON().content?.[0]
}

function makeEditor() {
  return new Editor({
    element: document.body,
    extensions: [
      StarterKit.configure({ heading: { levels: [1, 2, 3] } }),
      TaskList,
      TaskItem.configure({ nested: true }),
      TaskInputRule,
    ],
    content: { type: 'doc', content: [{ type: 'paragraph' }] },
  })
}

/** 在文档开头（段落内 pos 1）插入文本并触发 input rule。 */
function typeText(editor: Editor, text: string) {
  editor.commands.insertContentAt(1, text, { applyInputRules: true })
}

describe('TaskInputRule', () => {
  let editor: Editor

  beforeEach(() => {
    document.body.innerHTML = ''
    editor = makeEditor()
  })

  it('打 "- [ ] " 时创建未勾选任务列表项', async () => {
    typeText(editor, '- [ ] ')
    await flushInputRules()

    const block = firstBlock(editor)
    expect(block.type).toBe('taskList')
    const taskItem = block.content?.[0]
    expect(taskItem?.type).toBe('taskItem')
    expect(taskItem?.attrs?.checked).toBe(false)
  })

  it('打 "- [x] " 时创建已勾选任务列表项', async () => {
    typeText(editor, '- [x] ')
    await flushInputRules()

    const block = firstBlock(editor)
    expect(block.type).toBe('taskList')
    const taskItem = block.content?.[0]
    expect(taskItem?.type).toBe('taskItem')
    expect(taskItem?.attrs?.checked).toBe(true)
  })

  it('打 "- [X] "（大写 X）也算已勾选', async () => {
    typeText(editor, '- [X] ')
    await flushInputRules()

    const block = firstBlock(editor)
    const taskItem = block.content?.[0]
    expect(taskItem?.type).toBe('taskItem')
    expect(taskItem?.attrs?.checked).toBe(true)
  })

  it('打 "- 文本" 时是普通无序列表(不误判为任务)', async () => {
    // 模拟逐段输入:先 "- " 触发 BulletList,再补 "文本"(真实用户是逐字符)
    typeText(editor, '- ')
    await flushInputRules()
    typeText(editor, '文本')
    await flushInputRules()

    const block = firstBlock(editor)
    expect(block.type).toBe('bulletList')
    expect(block.content?.[0].type).toBe('listItem')
  })

  it('打 "* [ ] " 时是普通无序列表(星号 marker 不识别成任务)', async () => {
    // 用户选定仅识别减号 marker,* 仍走 BulletList
    typeText(editor, '* ')
    await flushInputRules()
    typeText(editor, '[ ] ')
    await flushInputRules()

    const block = firstBlock(editor)
    expect(block.type).toBe('bulletList')
    expect(block.content?.[0].type).toBe('listItem')
  })
})

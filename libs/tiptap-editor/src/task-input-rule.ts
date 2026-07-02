import { Extension, wrappingInputRule } from '@tiptap/core'

/**
 * 匹配行首的 `- [ ]` / `- [x]`(减号 marker + 空格 + 方括号 + 空格)。
 *
 * 为什么需要它:Tiptap 官方 TaskItem 的 input rule 只匹配 `[ ] `(不含 marker),
 * 而 StarterKit 的 BulletList input rule `/^\s*([-+*])\s$/` 在用户打 `- ` 时就抢先触发,
 * 把这行变成无序列表;后续的 `[ ]` 沦为字面文本,getMarkdown 序列化时还会转义成 `\[ \]`。
 * 本扩展用更高 priority 抢在 BulletList 之前,完整匹配 `- [ ] ` 直接创建任务列表。
 *
 * 仅识别减号 marker(与 GitHub/Typora 习惯一致);`*` `+` 仍走 BulletList。
 */
const taskInputRegex = /^(\s*-\s)\[( |x|X)\]\s$/

/**
 * 让手动输入 `- [ ]` / `- [x]` 直接创建任务列表项的扩展。
 *
 * 通过 `priority: 1000` 抢在 BulletList(默认 priority 100)之前执行 input rule。
 * `wrappingInputRule` 指向 taskItem 类型,ProseMirror 的 findWrapping 会自动补齐
 * `paragraph → taskList > taskItem` 的两层包裹路径(taskItem 的 schema 要求它在 taskList 内)。
 *
 * Enter 续行复用 TaskItem 内置的 splitListItem(新建项 checked=false,空项再 Enter 退出),
 * 这里不重复实现。
 */
export const TaskInputRule = Extension.create({
  name: 'taskInputRule',

  // 高于 BulletList 默认的 100,确保本扩展的 input rule 先匹配。
  priority: 1000,

  addInputRules() {
    return [
      wrappingInputRule({
        find: taskInputRegex,
        type: this.editor.schema.nodes.taskItem,
        getAttributes: (match) => ({
          checked: match[2].toLowerCase() === 'x',
        }),
      }),
    ]
  },
})

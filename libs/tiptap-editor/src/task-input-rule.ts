import { Extension } from '@tiptap/core'
import type { Node, Fragment } from '@tiptap/pm/model'
import { Plugin, PluginKey, TextSelection } from '@tiptap/pm/state'

/**
 * 让手动输入 `- [ ]` / `- [x]` 创建任务列表。
 *
 * 为什么不能用 input rule:StarterKit 的 BulletList input rule `/^\s*([-+*])\s$/`
 * 在用户打 `- `(第 2 个字符)时就触发,把这行变成 bulletList > listItem;
 * 之后用户继续打 `[ ]`,光标已在 listItem 内,任何 input rule 都无法把 listItem
 * 升级成 taskItem(input rule 只能在匹配时包裹当前块,不能跨节点类型转换)。
 * 提高 priority 也无效——BulletList 在更短的文本(2 字符)上就抢先,而我们的模式
 * 需要 5 字符才完整。
 *
 * 解法:用 appendTransaction 监听文档变化。当 BulletList 触发后,用户在 listItem 里
 * 打出 `[ ] ` / `[x] ` 前缀时,把该 bulletList 整体替换成 taskList,对应的 listItem
 * 转成带 checked 属性的 taskItem,并删除已识别的 `[ ] ` 文本前缀。
 *
 * 仅识别减号 marker 的 bulletList(用户选定),Enter 续行复用 TaskItem 内置的
 * splitListItem,这里不处理。
 */

const pluginKey = new PluginKey('taskListAutoConvert')

/** 匹配 listItem 内刚打出的 `[ ] ` / `[x] ` / `[X] ` 前缀。 */
const taskPrefixRegex = /^\[([ xX])\]\s/

export const TaskInputRule = Extension.create({
  name: 'taskInputRule',

  addProseMirrorPlugins() {
    const { schema } = this.editor
    const bulletListType = schema.nodes.bulletList
    const listItemType = schema.nodes.listItem
    const taskListType = schema.nodes.taskList
    const taskItemType = schema.nodes.taskItem

    // 缺任一相关节点类型(如未启用 TaskList),则插件空转。
    if (!bulletListType || !listItemType || !taskListType || !taskItemType) {
      return []
    }

    return [
      new Plugin({
        key: pluginKey,
        appendTransaction: (transactions, _oldState, newState) => {
          // 仅在文档实际变化时检查;非文档变化(选区移动)直接跳过。
          const docChanged = transactions.some((tr) => tr.docChanged)
          if (!docChanged) return null

          const { selection, tr } = newState
          // 仅处理光标(非选区)输入场景。
          if (!selection.empty) return null
          const $from = selection.$from

          // 向上查找最近的 listItem 祖先,以及它是否在 bulletList 内。
          let listItemDepth = -1
          for (let depth = $from.depth; depth > 0; depth--) {
            if ($from.node(depth).type === listItemType) {
              listItemDepth = depth
              break
            }
          }
          if (listItemDepth < 0) return null

          const listItem = $from.node(listItemDepth)
          // listItem 必须直接位于 bulletList 内(排除已是 taskList 的情况)。
          const listParentDepth = listItemDepth - 1
          if (listParentDepth < 1) return null
          const listParent = $from.node(listParentDepth)
          if (listParent.type !== bulletListType) return null

          // 读 listItem 第一个子节点(应为段落)的文本。
          const firstChild = listItem.firstChild
          if (!firstChild || !firstChild.isTextblock) return null
          const text = firstChild.textContent
          const match = text.match(taskPrefixRegex)
          if (!match) return null

          // 命中:把整个 bulletList 替换成 taskList,逐个 listItem 转成 taskItem。
          // 单个 listItem 命中即升级整个列表(用户可在每一行独立打 [ ] 控制 checked)。
          const checked = match[1].toLowerCase() === 'x'

          // 构造新的 taskList:把 bulletList 的每个 listItem 转成 taskItem。
          // 当前命中的 listItem 删除 `[ ] ` 前缀并设 checked;其余 listItem 默认未勾选。
          const listPos = $from.start(listParentDepth) - 1 // bulletList 节点的文档位置
          const newItems: Node[] = []
          // 记录命中项在 bulletList 中的序号(用于替换后定位光标)。
          let matchedIndex = -1
          let i = 0
          listParent.forEach((itemNode) => {
            const isMatched = itemNode === listItem
            if (isMatched) matchedIndex = i
            // 复用原 listItem 的子节点(段落等),命中的去掉前缀文本。
            let children = itemNode.content
            if (isMatched) {
              children = stripPrefix(children, match[0].length)
            }
            newItems.push(
              taskItemType.create(
                { checked: isMatched ? checked : false },
                children,
              ),
            )
            i++
          })

          const newTaskList = taskListType.create(null, newItems)
          const replaceTr = tr.replaceWith(listPos, listPos + listParent.nodeSize, newTaskList)

          // 显式重设光标:整段 replaceWith 后,ProseMirror 的选区映射无法把原 listItem
          // 内的光标正确映射到新 taskItem 内(默认落到替换区域之后=下一行)。
          // 这里算出命中的 taskItem 在新文档中的位置,把光标设到其段落文本末尾。
          const newMatchedItem = newTaskList.child(matchedIndex)
          // taskItem 的第一个子节点是段落(textblock);光标落在段落内容末尾。
          const itemPos = listPos + 1 // taskList 起始后进入第一个 taskItem
          let cursorPos = itemPos
          for (let k = 0; k < matchedIndex; k++) {
            cursorPos += newTaskList.child(k).nodeSize
          }
          // cursorPos 现在指向命中 taskItem 的起始;+1 进入其第一个段落,
          // +段落文本长度 = 段落末尾(用户继续输入的位置)。
          const paragraph = newMatchedItem.firstChild
          const paragraphTextLen = paragraph ? paragraph.content.size : 0
          cursorPos += 1 + paragraphTextLen
          replaceTr.setSelection(TextSelection.near(replaceTr.doc.resolve(cursorPos)))

          return replaceTr
        },
      }),
    ]
  },
})

/**
 * 从段落等内容片段的开头文本节点里删掉前缀字符数。
 * 输入规则识别的 `[ ] ` 是段落开头的纯文本,删掉对应字符即可。
 */
function stripPrefix(content: Fragment, len: number) {
  // content 是 Fragment;用 cut 切除开头 len 个字符位置。
  // Fragment.cut(from) 返回从 from 开始的子片段。
  return content.cut(len)
}

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
          // 跳过由本插件产生的 transaction,避免重入(Enter 等操作产生的文档变化
          // 不应再次触发升级判断,否则可能覆盖 splitListItem 的结果)。
          const ownTr = transactions.some((tr) => tr.getMeta(pluginKey))
          if (ownTr) return null

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
            // 复用原 listItem 的子节点(段落等);命中的去掉段落文本前缀。
            // strip 必须作用在「段落内部的 content」上,而非 listItem 的 content:
            // 后者把段落节点当成原子,cut(N) 会切进段落的标签边界,产生畸形文档,
            // 导致后续 splitListItem 行为异常(空项 Enter 不退出)。
            let children = itemNode.content
            if (isMatched) {
              const para = itemNode.firstChild!
              const stripped = para.type.create(
                para.attrs,
                stripPrefix(para.content, match[0].length),
              )
              // 用切除前缀后的段落替换第一个子节点;保留其余嵌套块(若有)。
              children = itemNode.content.replaceChild(0, stripped)
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

          // 显式重设光标:整段 replaceWith 后,选区映射会把光标甩到替换区域之后。
          // 基于新文档(replaceTr.doc)精确计算命中 taskItem 的段落末尾位置:
          //   taskList 起始 listPos → 内容 listPos+1 → 偏移 matchedIndex 个 taskItem → +1 进段落 → +段落 size
          const resolvedList = replaceTr.doc.resolve(listPos + 1)
          const taskListNode = resolvedList.node()
          let offset = 0
          for (let k = 0; k < matchedIndex && k < taskListNode.childCount; k++) {
            offset += taskListNode.child(k).nodeSize
          }
          const hitItem = taskListNode.child(matchedIndex)
          const hitParaTextLen = hitItem.firstChild ? hitItem.firstChild.content.size : 0
          // 位置拆解:taskList 节点在 listPos;+1 进内容(第一个 taskItem 节点位置);
          // +offset 到命中 taskItem 节点;+1 跨 taskItem 开标签进其内容(段落节点位置);
          // +1 跨段落开标签进文本;+文本长度到末尾。
          const cursorPos = listPos + 1 + offset + 1 + 1 + hitParaTextLen
          replaceTr.setSelection(TextSelection.near(replaceTr.doc.resolve(cursorPos)))
          // 标记为本插件产生的 transaction,防止 appendTransaction 重入。
          replaceTr.setMeta(pluginKey, { converted: true })

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

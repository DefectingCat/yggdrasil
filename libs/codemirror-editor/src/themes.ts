// @catppuccin/codemirror 提供现成的 Catppuccin 主题 Extension，
// 与项目 themes/ 下的 Catppuccin Latte/Mocha .tmTheme 视觉一致。
import { catppuccinLatte, catppuccinMocha } from '@catppuccin/codemirror';
import type { Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';

export type ThemeName = 'light' | 'dark';

/**
 * 覆盖 CodeMirror core 内置 base theme 的两处问题：
 *
 * 1. `.cm-gutters` 默认背景：core 的 `&light .cm-gutters`（`#f5f5f5`）/ `&dark`
 *    （`#333338`）特异性高于 catppuccin 的 `.cm-gutters`，catppuccin 的 base 背景被
 *    压制，行号列与代码区背景不一致。用 `!important` 强制透明，继承 editor base 色。
 *
 * 2. `.cm-editor` 默认不撑满父容器：core 的 `&` 没设 height/flex，编辑器只占内容
 *    高度。`height: 100%` 在父容器只有 min-height 时会塌缩（CSS：百分比高度需要
 *    父元素有明确 height）。改用 `flex: 1`，配合父容器 `display: flex`，编辑器才能
 *    真正填满，避免「有内容的上半部分」与「空白下半部分」背景割裂。
 */
const gutterBackgroundOverride: Extension = EditorView.theme({
  '&': {
    flex: '1 1 0',
    minHeight: '0',
  },
  '.cm-gutters': {
    backgroundColor: 'transparent !important',
  },
});

/**
 * 修复 CodeMirror 折叠图标（fold gutter 的 ▾ / ▸）垂直不居中。
 *
 * core 的 baseTheme（@codemirror/language）只给 `.cm-foldGutter span` 设了
 * `padding`/`cursor`，没设 `display`/`vertical-align`。折叠图标是个 <span>，
 * 装在 `.cm-gutterElement` 里（高度被 core 固定为行高的格子），默认按基线对齐，
 * 图标贴在格子底部而非中央。
 *
 * editor.ts 已把图标字符从基线不稳的 `⌄`(U+2304)/`›`(U+203A) 换成几何三角形
 * `▾`(U+25BE)/`▸`(U+25B8)——字形本身在字符框内居中；再配合这里的 flex 居中，
 * 三角稳稳落在行框中央。只作用到 `.cm-foldGutter`，不影响行号列（行号文本仍按
 * 基线，正常可读）。
 */
const foldGutterCenterOverride: Extension = EditorView.theme({
  '.cm-foldGutter .cm-gutterElement': {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
});

/** 根据主题名返回对应的 CodeMirror 主题 Extension。 */
export function themeExtension(name: ThemeName): Extension {
  const catppuccin = name === 'light' ? catppuccinLatte : catppuccinMocha;
  return [catppuccin, gutterBackgroundOverride, foldGutterCenterOverride];
}

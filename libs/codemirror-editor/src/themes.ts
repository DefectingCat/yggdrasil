// @catppuccin/codemirror 提供现成的 Catppuccin 主题 Extension，
// 与项目 themes/ 下的 Catppuccin Latte/Mocha .tmTheme 视觉一致。
import { catppuccinLatte, catppuccinMocha } from '@catppuccin/codemirror';
import { EditorView } from '@codemirror/view';
import type { Extension } from '@codemirror/state';

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

/** 根据主题名返回对应的 CodeMirror 主题 Extension。 */
export function themeExtension(name: ThemeName): Extension {
  const catppuccin = name === 'light' ? catppuccinLatte : catppuccinMocha;
  return [catppuccin, gutterBackgroundOverride];
}

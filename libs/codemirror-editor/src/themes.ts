// @catppuccin/codemirror 提供现成的 Catppuccin 主题 Extension，
// 与项目 themes/ 下的 Catppuccin Latte/Mocha .tmTheme 视觉一致。
import { catppuccinLatte, catppuccinMocha } from '@catppuccin/codemirror';
import { EditorView } from '@codemirror/view';
import type { Extension } from '@codemirror/state';

export type ThemeName = 'light' | 'dark';

/**
 * 覆盖 CodeMirror core 内置 base theme 给 `.cm-gutters` 设的默认背景。
 *
 * CodeMirror core 的 base theme 用 `&light .cm-gutters` / `&dark .cm-gutters`
 * （特异性 `.cm-editor.cm-light .cm-gutters`）注入了一个浅灰/深灰背景
 * （light `#f5f5f5`、dark `#333338`），优先级高于 catppuccin 的 `.cm-gutters`
 * 覆盖，导致行号列与编辑器 content（catppuccin base 色）背景不一致，产生割裂。
 *
 * 这里用 `!important` 强制 gutter 背景透明，使其继承 `.cm-editor` 的 base 色，
 * 行号区与代码区融为一体。
 */
const gutterBackgroundOverride: Extension = EditorView.theme({
  '.cm-gutters': {
    backgroundColor: 'transparent !important',
  },
});

/** 根据主题名返回对应的 CodeMirror 主题 Extension。 */
export function themeExtension(name: ThemeName): Extension {
  const catppuccin = name === 'light' ? catppuccinLatte : catppuccinMocha;
  return [catppuccin, gutterBackgroundOverride];
}

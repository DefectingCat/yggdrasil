// @catppuccin/codemirror 提供现成的 Catppuccin 主题 Extension，
// 与项目 themes/ 下的 Catppuccin Latte/Mocha .tmTheme 视觉一致。
import { catppuccinLatte, catppuccinMocha } from '@catppuccin/codemirror';
import type { Extension } from '@codemirror/state';

export type ThemeName = 'light' | 'dark';

/** 根据主题名返回对应的 CodeMirror 主题 Extension。 */
export function themeExtension(name: ThemeName): Extension {
  return name === 'light' ? catppuccinLatte : catppuccinMocha;
}

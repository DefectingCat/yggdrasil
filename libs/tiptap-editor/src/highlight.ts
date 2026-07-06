import { createLowlight, common } from 'lowlight';

const base = createLowlight(common);

/**
 * 从完整 fence info string 提取语言名（首个 token）。
 *
 * 与后端 parse_fence_info（src/api/code_runner/languages.rs:80）对齐：
 * info string 形如 `python runnable {"timeout_secs":10}`，语言是首个空白分隔的 token。
 */
export function extractLang(info: string): string {
  return info.trim().split(/\s+/)[0] || '';
}

/**
 * lowlight 实例，包装 highlight 方法以处理完整 info string。
 *
 * CodeBlockLowlight 取 `block.node.attrs.language`（如 `python runnable {...}`）
 * 直接调 `lowlight.highlight(language, code)`，但 lowlight 只认 `python` 这个 token。
 * wrapper 先 extractLang 提取首 token，再高亮，使 runnable 块也能按正确语言着色。
 *
 * 普通 code block（language='python'）也兼容：extractLang('python') → 'python'。
 */
export const lowlight = {
  ...base,
  highlight(language: string, value: string) {
    return base.highlight(extractLang(language), value);
  },
};

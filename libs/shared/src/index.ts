/**
 * 跨 IIFE 库共享的类型、常量与工具函数。
 *
 * 这些库各自打包成独立 IIFE（不能 import 彼此），但它们的主题切换逻辑
 * 需要约定相同的事件名与类型定义。本包作为单一真相源，由各库以 workspace
 * dependency 引入，Vite 构建时 inline 进各自的 IIFE bundle。
 */

/** 主题名称：亮色 / 暗色。 */
export type ThemeName = 'light' | 'dark';

/**
 * 主题切换事件名。
 *
 * yggdrasil-core 在切换主题时 dispatch 此事件；codemirror-editor / xterm-terminal
 * 监听它以热切换编辑器主题。所有库必须用同一个字符串字面量。
 */
export const THEME_CHANGE_EVENT = 'yggdrasil:theme-change';

/**
 * 检测用户是否在系统层面启用了「减少动态效果」（prefers-reduced-motion）。
 *
 * 用于决定是否跳过 View Transitions / 动画，直接切换。
 */
export function prefersReducedMotion(): boolean {
  return !!window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

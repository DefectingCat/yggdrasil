/**
 * 圆形展开主题切换动画(View Transitions API)。
 *
 * 采用纯 CSS 配合 CSS 变量的实现方案。
 * 核心策略:始终让"暗色层"在上方,通过 clip-path 揭示下方的"亮色层"。
 * - 亮 -> 暗: NEW 是暗色(在上方),从小圆扩大(`tt-expand`)覆盖底部的 OLD。
 * - 暗 -> 亮: OLD 是暗色(在上方),从大圆缩小(`tt-shrink`)揭开底部的 NEW。
 *
 * 相比 WAAPI 或动态注入 <style>,这种方式完全没有特异性冲突、DOM 残留或
 * API 优先级 bug,是目前最稳定的 VT 主题切换方案。
 */

/**
 * 主题切换自定义事件名。
 *
 * 在 VT 回调内(NEW 快照捕获前)同步 dispatch,通知 CodeMirror / xterm 等
 * 命令式换肤的组件同步调 setTheme——它们的背景色不随 .dark class 翻转,
 * 必须在快照前显式换肤,否则圆形展开扫过时看不到变化(OLD/NEW 同色)。
 *
 * 事件 detail: `{ isDark: boolean }`。
 *
 * 各编辑器包(codemirror-editor / xterm-terminal)是独立 IIFE,不 import 本包,
 * 故各自用同名 string literal 订阅;本常量仅用于本包内部 + 测试断言一致性。
 */
export const THEME_CHANGE_EVENT = 'yggdrasil:theme-change';

function prefersReducedMotion(): boolean {
  return !!window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

function maxCornerDistance(x: number, y: number): number {
  const w = window.innerWidth;
  const h = window.innerHeight;
  const corners = [
    [0, 0],
    [w, 0],
    [0, h],
    [w, h],
  ];
  let max = 0;
  for (const [cx, cy] of corners) {
    const d = Math.hypot(cx - x, cy - y);
    if (d > max) max = d;
  }
  return max;
}

function applyDarkClass(isDark: boolean): void {
  const html = document.documentElement;
  if (isDark) {
    html.classList.add('dark');
  } else {
    html.classList.remove('dark');
  }
}

/**
 * 同步通知命令式换肤的组件(CodeMirror / xterm)切换主题。
 *
 * CustomEvent 的 dispatch 是同步的:listener 在本函数返回前执行完毕,
 * 故编辑器的 setTheme(reconfigure / options.theme =) 在调用方继续前已完成。
 * 这对 VT 至关重要——必须在 NEW 快照捕获前完成换肤,否则快照里仍是旧色。
 *
 * 幂等:与 Dioxus use_effect 驱动的 set_theme 并存,重复设置相同主题是 no-op。
 */
function notifyThemeChange(isDark: boolean): void {
  window.dispatchEvent(new CustomEvent(THEME_CHANGE_EVENT, { detail: { isDark } }));
}

/**
 * 直接设置 <html> 的 dark class（设置语义，非翻转）。
 *
 * 用于跟随系统偏好变化时的后台同步：系统偏好变化是后台事件，View Transitions
 * 在此上下文下动画不可靠（实测圆形展开不显示，仅瞬切），故跟随系统场景不走
 * startThemeTransition 的 VT 路径，改用此函数直接同步 class，做无动画的瞬切。
 * 手动点击主题按钮仍走 startThemeTransition，保留圆形展开动画。
 *
 * 同步 dispatch 主题变更事件,让命令式换肤的编辑器跟随系统偏好瞬切
 * (与 Dioxus use_effect 幂等共存,后者作兜底)。
 */
export function applyResolvedTheme(isDark: boolean): void {
  notifyThemeChange(isDark);
  applyDarkClass(isDark);
}

export function startThemeTransition(x: number, y: number): void {
  const html = document.documentElement;
  const isDark = !html.classList.contains('dark');

  const hasVT = typeof document.startViewTransition === 'function';
  const reduced = prefersReducedMotion();

  if (!hasVT || reduced) {
    // 降级路径:无 VT 动画,同步换肤 + 翻 class(瞬切)。
    // 同样 dispatch 事件,保持与主路径对称(编辑器不依赖动画存在与否)。
    notifyThemeChange(isDark);
    applyDarkClass(isDark);
    return;
  }

  const maxR = maxCornerDistance(x, y);

  // 注入动画需要的 CSS 变量
  html.style.setProperty('--tt-x', `${x}px`);
  html.style.setProperty('--tt-y', `${y}px`);
  html.style.setProperty('--tt-r', `${maxR}px`);

  // 禁用所有 CSS transition,确保 VT 截图是最终颜色
  html.classList.add('is-theme-transitioning');

  const vt = document.startViewTransition(() => {
    // ★ 关键:先 dispatch 事件让编辑器同步换肤,再翻 .dark class。
    // 顺序不能反——编辑器换肤 + class 翻转必须被同一个 getComputedStyle
    // reflow 捕获进 NEW 快照。若先翻 class 后换肤,reflow 可能漏掉编辑器。
    notifyThemeChange(isDark);
    applyDarkClass(isDark);
    // 强制同步样式重算:确保 body 的 background-color 解析为目标值,
    // 同时 flush 编辑器的同步换肤(CodeMirror <style> / xterm inline bg)。
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    getComputedStyle(document.body).backgroundColor;
  });

  vt.ready.catch(() => {});

  vt.finished.finally(() => {
    html.classList.remove('is-theme-transitioning');
    // 清理 CSS 变量
    html.style.removeProperty('--tt-x');
    html.style.removeProperty('--tt-y');
    html.style.removeProperty('--tt-r');
  });
}

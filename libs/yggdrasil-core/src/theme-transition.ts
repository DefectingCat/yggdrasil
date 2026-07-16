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

import { prefersReducedMotion, THEME_CHANGE_EVENT } from '@yggdrasil/shared';

/**
 * 主题切换自定义事件。
 *
 * 在 VT 回调内(NEW 快照捕获前)同步 dispatch,通知 CodeMirror / xterm 等
 * 命令式换肤的组件同步调 setTheme——它们的背景色不随 .dark class 翻转,
 * 必须在快照前显式换肤,否则圆形展开扫过时看不到变化(OLD/NEW 同色)。
 *
 * 事件 detail: `{ isDark: boolean }`。
 *
 * 事件名与 prefersReducedMotion 由 @yggdrasil/shared 统一定义,各 IIFE 库
 * (codemirror-editor / xterm-terminal / lightbox)共享同一真相源。
 */
export { THEME_CHANGE_EVENT };

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
 * 主题切换 registry:命令式 / 异步换肤组件注册的回调。
 *
 * 与 THEME_CHANGE_EVENT 事件并存:
 * - 事件(CustomEvent):同步 dispatch,供 CodeMirror / xterm 等同步换肤组件用
 *   (它们在 listener 里同步 setTheme,被同一 reflow 捕获进 NEW 快照)。事件拿不到
 *   listener 返回值,fire-and-forget。
 * - registry:回调**可返回 Promise**,调用方(如 VT callback)能 await 它,等异步换肤
 *   组件(如 mermaid 的 render())完成。这是让异步渲染内容参与 VT 动画的关键——
 *   mermaid.render 是异步的,必须在 VT 拍 NEW 快照前完成,否则快照里仍是旧图。
 */
const themeChangeCallbacks = new Set<(isDark: boolean) => Promise<void> | void>();

/** 注册主题切换回调,返回取消注册函数。回调可返回 Promise,调用方会等待它。 */
export function onThemeChange(cb: (isDark: boolean) => Promise<void> | void): () => void {
  themeChangeCallbacks.add(cb);
  return () => themeChangeCallbacks.delete(cb);
}

/**
 * 通知命令式换肤的组件(CodeMirror / xterm / mermaid)切换主题。
 *
 * 双通道:
 * 1. 同步 dispatch THEME_CHANGE_EVENT(给 CodeMirror / xterm,它们在 listener 内
 *    同步 setTheme,被同一 reflow 捕获进 NEW 快照)。
 * 2. 遍历 registry 调每个 cb,收集返回的 Promise,返回聚合 Promise(VT callback
 *    await 它以等 mermaid 等异步换肤完成)。同步组件不返回值,聚合自动忽略。
 *
 * 返回聚合 Promise,调用方可选 await(走 VT 的路径必须 await,瞬切路径可不 await)。
 */
function notifyThemeChange(isDark: boolean): Promise<void> {
  window.dispatchEvent(new CustomEvent(THEME_CHANGE_EVENT, { detail: { isDark } }));
  const promises: Promise<void>[] = [];
  themeChangeCallbacks.forEach((cb) => {
    try {
      const ret = cb(isDark);
      if (ret) promises.push(ret.catch(() => {})); // 单个失败不中断聚合
    } catch {
      // 同步抛错的 cb 忽略,不中断其他回调
    }
  });
  return Promise.all(promises).then(() => {});
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
    // 同样通知换肤(不 await,保持瞬切语义;mermaid 等异步组件后台重渲染)。
    void notifyThemeChange(isDark);
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

  const vt = document.startViewTransition(async () => {
    // ★ 关键:先通知换肤(同步 dispatch 事件让编辑器同步换肤 + 收集 registry 的
    // 异步 Promise),再翻 .dark class。顺序不能反——编辑器换肤 + class 翻转必须被
    // 同一个 getComputedStyle reflow 捕获进 NEW 快照。
    const asyncWork = notifyThemeChange(isDark);
    applyDarkClass(isDark);
    // 强制同步样式重算:确保 body 的 background-color 解析为目标值,
    // 同时 flush 编辑器的同步换肤(CodeMirror <style> / xterm inline bg)。
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    getComputedStyle(document.body).backgroundColor;
    // ★ 等 registry 里异步换肤组件(mermaid.render)完成。callback 返回 Promise 时,
    // 浏览器等它 resolve 才拍 NEW 快照、播圆形扩散——这样快照里已是新主题流程图。
    await asyncWork;
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

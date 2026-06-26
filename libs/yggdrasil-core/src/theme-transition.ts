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

function prefersReducedMotion(): boolean {
  return (
    !!window.matchMedia &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  );
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

export function startThemeTransition(x: number, y: number): void {
  const html = document.documentElement;
  const isDark = !html.classList.contains('dark');

  const hasVT = typeof document.startViewTransition === 'function';
  const reduced = prefersReducedMotion();

  if (!hasVT || reduced) {
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
    applyDarkClass(isDark);
    // 强制同步样式重算:确保 body 的 background-color 解析为目标值
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

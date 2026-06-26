/**
 * 圆形展开主题切换动画(View Transitions API)。
 *
 * 点击按钮时,新主题页面从点击点 (x,y) 以圆形向外展开覆盖全屏。
 * 流程:设 CSS 圆心/半径变量 → startViewTransition(同步 toggle dark class)
 * → CSS @keyframes 用 clip-path: circle() 展开 ::view-transition-new(root)。
 *
 * 降级:无 startViewTransition 或 prefers-reduced-motion 时瞬切 dark class。
 * 重入保护:动画进行中(transitioning=true)忽略后续点击。
 */

let transitioning = false;

function prefersReducedMotion(): boolean {
  return (
    !!window.matchMedia &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  );
}

/** 视口四角到 (x,y) 的最大欧氏距离,作为圆形展开半径(保证完全覆盖视口)。 */
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

export function startThemeTransition(x: number, y: number, isDark: boolean): void {
  if (transitioning) return;
  transitioning = true;

  const hasVT = typeof document.startViewTransition === 'function';
  const reduced = prefersReducedMotion();

  // 降级路径:瞬切。body 的 .3s 背景过渡仍给柔和淡入。
  if (!hasVT || reduced) {
    applyDarkClass(isDark);
    transitioning = false;
    return;
  }

  // 主路径:设圆心与半径,启动 VT。
  const html = document.documentElement;
  html.style.setProperty('--tt-x', `${x}px`);
  html.style.setProperty('--tt-y', `${y}px`);
  html.style.setProperty('--tt-max-r', `${maxCornerDistance(x, y)}px`);

  const vt = document.startViewTransition(() => {
    applyDarkClass(isDark);
  });

  // vt.finished 在 skipTransition 或页面跳转时会 reject(属预期),用 .catch 吞掉
  // 以避免 unhandled rejection 刷控制台;无论 resolve/reject 都复位重入标志。
  vt.finished
    .catch(() => {})
    .finally(() => {
      transitioning = false;
    });
}

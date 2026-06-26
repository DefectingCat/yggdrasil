/**
 * 圆形展开主题切换动画(View Transitions API)。
 *
 * 点击按钮时,新主题页面从点击点 (x,y) 以圆形向外展开覆盖全屏。
 *
 * 设计要点:
 * 1. 所有 VT 伪元素样式 **在 startViewTransition 之前** 预注入到 <head>,
 *    确保浏览器创建 ::view-transition-old/new 时样式已就绪,无时序竞态。
 *    (旧方案在 vt.ready 中才注入,存在一帧 NEW 层无动画全覆盖的间隙。)
 * 2. VT 回调中 toggle dark class 后, 调用 getComputedStyle 强制同步样式
 *    重算,确保浏览器截取的 NEW 快照反映最终颜色,不受 CSS transition 影响。
 * 3. 通过 html.is-theme-transitioning 全局 class 禁用所有 CSS transition,
 *    防止 body 的 background-color 0.3s 过渡干扰。
 *
 * 降级:无 startViewTransition 或 prefers-reduced-motion 时瞬切 dark class。
 */

let prevStyleEl: HTMLStyleElement | null = null;

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
  // 目标主题从 DOM 现状推导,不依赖外部传入——避免与 Rust Signal 状态不同步
  // 导致方向错乱(isDark 传反而 toggle 成 no-op,新旧快照一样看不到动画)。
  const isDark = !html.classList.contains('dark');
  console.log('[tt] ENTER', { x, y, isDark, domHasDark: html.classList.contains('dark') });

  const hasVT = typeof document.startViewTransition === 'function';
  const reduced = prefersReducedMotion();

  if (!hasVT || reduced) {
    console.log('[tt] DEGRADE');
    applyDarkClass(isDark);
    return;
  }

  const maxR = maxCornerDistance(x, y);

  // ── 1. 预注入 VT 伪元素样式(在 startViewTransition 之前) ──
  // 移除上一次注入的 style,避免堆积。
  if (prevStyleEl) {
    prevStyleEl.remove();
  }
  const name = `tt-${Date.now()}`;
  const style = document.createElement('style');
  style.textContent = `
    @keyframes ${name} {
      from { clip-path: circle(0px at ${x}px ${y}px); }
      to   { clip-path: circle(${maxR}px at ${x}px ${y}px); }
    }
    ::view-transition-old(root) {
      animation: none !important;
      mix-blend-mode: normal;
    }
    ::view-transition-new(root) {
      animation: ${name} 0.4s ease-out !important;
      mix-blend-mode: normal;
    }
  `;
  document.head.appendChild(style);
  prevStyleEl = style;

  // ── 2. 禁用所有 CSS transition,确保截图是最终颜色 ──
  html.classList.add('is-theme-transitioning');

  // ── 3. 启动 View Transition ──
  const vt = document.startViewTransition(() => {
    console.log('[tt] CALLBACK set dark=', isDark);
    applyDarkClass(isDark);
    // 强制同步样式重算:确保浏览器截取 NEW 快照前,
    // body 的 background-color 已经解析为目标值。
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    getComputedStyle(document.body).backgroundColor;
  });

  // 样式已预注入,vt.ready 不再需要处理
  vt.ready
    .then(() => console.log('[tt] ready OK'))
    .catch(() => {});

  vt.finished
    .then(() => console.log('[tt] VT finished OK'))
    .catch((e) => console.log('[tt] VT REJECT:', e))
    .finally(() => {
      html.classList.remove('is-theme-transitioning');
    });
}

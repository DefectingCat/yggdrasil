/**
 * 圆形展开主题切换动画(View Transitions API)。
 *
 * 点击按钮时,新主题页面从点击点 (x,y) 以圆形向外展开覆盖全屏。
 *
 * 实现方式:启动 VT 后,在 vt.ready 注入一段 <style>,用硬编码坐标的
 * @keyframes 驱动 ::view-transition-new(root) 的 clip-path: circle()。
 * 不用 CSS 变量(插值不稳定)、不用 element.animate(方向性 bug)。
 *
 * 关键设计:所有 VT 伪元素样式(animation/mix-blend-mode)都在 JS 中
 * 动态注入,不预设在静态 CSS 里。静态 CSS 里的 animation: none 会让
 * ::view-transition-new 在注入 @keyframes 之前完全不透明地覆盖 old,
 * 导致暗→亮方向看不到动画(亮色 new 瞬间铺满,clip-path 从 0 展开
 * 时已经无区别可展示)。
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

  // 添加临时类，用于在 CSS 中禁用 transition，确保截图捕获最终状态
  html.classList.add('is-theme-transitioning');

  const vt = document.startViewTransition(() => {
    console.log('[tt] CALLBACK set dark=', isDark);
    applyDarkClass(isDark);
  });

  vt.ready
    .then(() => {
      // 移除上一次注入的 style,避免堆积。
      if (prevStyleEl) {
        prevStyleEl.remove();
      }
      // 注入硬编码坐标的 @keyframes + VT 伪元素样式。
      // 关键:old 和 new 的 animation/mix-blend-mode 必须在这里设置,
      // 不能预设在静态 CSS 里,否则会出现 new 在动画注入前完全可见的闪烁。
      const name = `tt-${Date.now()}`;
      const style = document.createElement('style');
      style.textContent = `
        @keyframes ${name} {
          from { clip-path: circle(0px at ${x}px ${y}px); }
          to { clip-path: circle(${maxR}px at ${x}px ${y}px); }
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
      console.log('[tt] ready, injected keyframes:', name);
    })
    .catch(() => {});

  vt.finished
    .then(() => console.log('[tt] VT finished OK'))
    .catch((e) => console.log('[tt] VT REJECT:', e))
    .finally(() => {
      // 动画完成后移除临时类
      html.classList.remove('is-theme-transitioning');
    });
}

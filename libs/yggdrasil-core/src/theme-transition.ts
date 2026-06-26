/**
 * 圆形展开主题切换动画(View Transitions API)。
 *
 * 点击按钮时,新主题页面从点击点 (x,y) 以圆形向外展开覆盖全屏。
 *
 * 实现方式:使用 Web Animations API 的 pseudoElement 选项,在 vt.ready 中
 * 直接对 ::view-transition-old/new(root) 伪元素创建动画。相比注入 <style>
 * + @keyframes 的旧方案,优势在于:
 *   - Web Animation 优先级高于 CSS 动画(包括 UA 默认的 fade-in/out),
 *     天然覆盖,无需 !important 或担心特异性冲突;
 *   - 每次调用产生独立的 Animation 对象,不存在 <style> 残留导致后续
 *     切换动画失效的问题;
 *   - 无需管理 prevStyleEl 的生命周期。
 *
 * mix-blend-mode: normal 通过 style.css 静态设置(不可动画属性)。
 * CSS transition 在 VT 期间通过 .is-theme-transitioning class 全局禁用。
 *
 * 降级:无 startViewTransition 或 prefers-reduced-motion 时瞬切 dark class。
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

  // 禁用所有 CSS transition,确保 VT 截图是最终颜色
  html.classList.add('is-theme-transitioning');

  const vt = document.startViewTransition(() => {
    console.log('[tt] CALLBACK set dark=', isDark);
    applyDarkClass(isDark);
    // 强制同步样式重算:确保 body 的 background-color 解析为目标值
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    getComputedStyle(document.body).backgroundColor;
  });

  vt.ready
    .then(() => {
      console.log('[tt] ready, animating clip-path via WAAPI');
      // CSS 已通过 animation:none + opacity:1 (!important) 锁定两层:
      // OLD 保持完全可见(暗色底图),NEW 保持完全不透明。
      // WAAPI 只需控制 NEW 的 clip-path 实现圆形展开。
      // script-created Animation 优先级高于 CSS animation(已被 none 禁用),
      // 只添加 clip-path 动画,不与 CSS 冲突。
      document.documentElement.animate(
        {
          clipPath: [
            `circle(0px at ${x}px ${y}px)`,
            `circle(${maxR}px at ${x}px ${y}px)`,
          ],
        },
        {
          duration: 400,
          easing: 'ease-out',
          pseudoElement: '::view-transition-new(root)',
        },
      );
    })
    .catch(() => {});

  vt.finished
    .then(() => console.log('[tt] VT finished OK'))
    .catch((e) => console.log('[tt] VT REJECT:', e))
    .finally(() => {
      html.classList.remove('is-theme-transitioning');
    });
}

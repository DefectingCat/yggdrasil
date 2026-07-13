/**
 * 锚点跳转：根据 URL hash 滚动到对应标题。
 *
 * 解决 SPA 异步取数时序问题：文章正文经 use_server_future 异步加载，骨架屏阶段
 * 标题 DOM 尚不存在，浏览器原生 fragment-scroll 找不到目标留在顶部。本函数在
 * 内容挂载后调用，弥补原生 scroll 丢失的窗口。
 *
 * 落点遮挡问题：直接访问 #hash 链接时，浏览器原生 fragment-scroll 会在我们
 * scrollToHash 之后触发，它不读 scroll-margin-top，把标题顶到 sticky header 下面。
 * 解法：用 requestAnimationFrame 延后两帧再做最终校正，确保我们的落点在原生
 * scroll 之后生效（原生 scroll 通常在下一帧完成）。
 *
 * CJK 编码要点：location.hash 返回百分号编码形式（%E4%B8%89-...），而标题 id
 * 属性是原始字符（id="三-五-零法则"）。必须先 decodeURIComponent 还原，getElementById
 * 才能匹配。双 fallback（先解码、后原始）兼容两种情况。
 */

import { scrollToHeading } from './scroll-to-heading';

export function scrollToHash(): void {
  const hash = window.location.hash.slice(1); // 去掉前导 #
  if (!hash) return;

  const id = decodeURIComponent(hash);
  const el = document.getElementById(id) ?? document.getElementById(hash);
  if (!el) return;

  // 立即先滚一次（覆盖骨架屏阶段原生 scroll 失败的窗口）。
  scrollToHeading(el, true);

  // 延后两帧再校正：直接访问 #hash 时浏览器原生 fragment-scroll 会在此期间触发，
  // 把标题顶到 header 下面（它不读 scroll-margin-top）。两帧后原生 scroll 已完成，
  // 用即时滚动（非 smooth，避免与刚才的 smooth 动画叠加抖动）把落点拉回正确位置。
  requestAnimationFrame(() => {
    requestAnimationFrame(() => scrollToHeading(el, false));
  });
}

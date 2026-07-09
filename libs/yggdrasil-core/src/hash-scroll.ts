/**
 * 锚点跳转：根据 URL hash 滚动到对应标题。
 *
 * 解决 SPA 异步取数时序问题：文章正文经 use_server_future 异步加载，骨架屏阶段
 * 标题 DOM 尚不存在，浏览器原生 fragment-scroll 找不到目标留在顶部。本函数在
 * 内容挂载后调用，弥补原生 scroll 丢失的窗口。
 *
 * CJK 编码要点：location.hash 返回百分号编码形式（%E4%B8%89-...），而标题 id
 * 属性是原始字符（id="三-五-零法则"）。必须先 decodeURIComponent 还原，getElementById
 * 才能匹配。双 fallback（先解码、后原始）兼容两种情况。
 */

export function scrollToHash(): void {
  const hash = window.location.hash.slice(1); // 去掉前导 #
  if (!hash) return;

  const id = decodeURIComponent(hash);
  const el = document.getElementById(id) ?? document.getElementById(hash);
  if (!el) return;

  el.scrollIntoView({ behavior: 'smooth', block: 'start' });
}

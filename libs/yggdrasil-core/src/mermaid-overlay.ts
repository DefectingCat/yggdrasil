/**
 * Mermaid 流程图点击放大浮层。
 *
 * 问题：复杂流程图（多 subgraph / 宽 LR 布局）在小屏被 useMaxWidth + max-width:100%
 * 压缩到无法阅读。本模块提供一个全屏浮层：点击图 → 克隆 SVG 按原始尺寸渲染 →
 * 支持拖拽平移、滚轮/双指缩放、双击切换适配/原始大小。
 *
 * 设计要点：
 * - SVG 是矢量，放大无损。克隆后剥离 max-width 约束，按 viewBox 原始尺寸渲染。
 * - pan/zoom 用 CSS transform（translate + scale），transform-origin: 0 0，
 *   数学用「屏幕坐标 ↔ 内容坐标」映射，缩放锚定光标/捏合中心点。
 * - Pointer Events 统一鼠标 + 触摸：1 指 = 平移，2 指 = 捏合缩放。
 * - 每页同时只允许一个浮层（单例 state）。
 * - 关闭：ESC / 点击背景 / 点击 ✕ 按钮。
 */

// ── 单例状态 ──

let overlay: HTMLDivElement | null = null;
let content: HTMLDivElement | null;
let svgClone: SVGSVGElement | null;

/** 内容原始尺寸（viewBox 像素），用于 fit-to-screen 计算。 */
let naturalW = 0;
let naturalH = 0;

/** 内容 (0,0) 锚定在视口的基准坐标（让 SVG 初始居中）。 */
let originX = 0;
let originY = 0;

// transform 状态
let scale = 1;
let tx = 0;
let ty = 0;

// 缩放上下限
const MIN_SCALE = 0.1;
const MAX_SCALE = 5;

// pointer 跟踪
const pointers = new Map<number, { x: number; y: number }>();
let panStart: { x: number; y: number; tx: number; ty: number } | null = null;
let pinchStart: { dist: number; cx: number; cy: number; scale: number; tx: number; ty: number } | null = null;

// body 滚动锁
let prevBodyOverflow = '';

// ── 工具函数 ──

function clamp(v: number, min: number, max: number): number {
  return Math.min(Math.max(v, min), max);
}

function applyTransform(): void {
  if (!content) return;
  content.style.transform = `translate(${tx}px, ${ty}px) scale(${scale})`;
}

/**
 * 计算让 SVG 居中且完整可见的初始 transform。
 * 基准 origin 让 SVG 自然左上角对齐视口居中：origin = viewportCenter - naturalSize/2。
 */
function fitToScreen(): void {
  const pad = 48;
  const vw = window.innerWidth - pad;
  const vh = window.innerHeight - pad;
  // 适配缩放：不放大超出原始尺寸（除非图比视口还小）
  scale = clamp(Math.min(vw / naturalW, vh / naturalH), MIN_SCALE, 1);
  originX = (window.innerWidth - naturalW) / 2;
  originY = (window.innerHeight - naturalH) / 2;
  tx = 0;
  ty = 0;
  if (content) {
    content.style.left = `${originX}px`;
    content.style.top = `${originY}px`;
  }
  applyTransform();
}

/**
 * 在屏幕坐标 (px, py) 处缩放：保持该点对应的内容点不变。
 *
 * 数学推导（transform-origin: 0 0）：
 *   屏幕坐标 = origin + translate + scale × 内容坐标
 *   内容坐标 = (屏幕坐标 - origin - translate) / scale
 *
 * 缩放后保持同一内容点在 (px, py)：
 *   px = origin + newTx + newScale × contentX
 *   newTx = px - origin - newScale × contentX
 */
function zoomAt(px: number, py: number, newScale: number): void {
  newScale = clamp(newScale, MIN_SCALE, MAX_SCALE);
  const cx = (px - originX - tx) / scale;
  const cy = (py - originY - ty) / scale;
  tx = px - originX - newScale * cx;
  ty = py - originY - newScale * cy;
  scale = newScale;
  applyTransform();
}

// ── 浮层 DOM 构建 ──

function buildOverlay(svg: SVGElement): void {
  overlay = document.createElement('div');
  overlay.className = 'mermaid-overlay';
  overlay.setAttribute('role', 'dialog');
  overlay.setAttribute('aria-modal', 'true');
  overlay.setAttribute('aria-label', '流程图放大查看');

  // 关闭按钮
  const closeBtn = document.createElement('button');
  closeBtn.className = 'mermaid-overlay-close';
  closeBtn.type = 'button';
  closeBtn.setAttribute('aria-label', '关闭');
  closeBtn.textContent = '✕';

  // SVG 容器（应用 transform）
  content = document.createElement('div');
  content.className = 'mermaid-overlay-content';

  // 克隆 SVG，剥离 max-width 约束，按 viewBox 原始尺寸渲染
  svgClone = svg.cloneNode(true) as SVGSVGElement;
  const vb = svgClone.viewBox?.baseVal;
  if (vb && vb.width > 0 && vb.height > 0) {
    naturalW = vb.width;
    naturalH = vb.height;
  } else {
    // 退化：读取 width/height 属性
    naturalW = svgClone.width.baseVal.value || 800;
    naturalH = svgClone.height.baseVal.value || 600;
  }
  svgClone.style.maxWidth = 'none';
  svgClone.style.width = `${naturalW}px`;
  svgClone.style.height = `${naturalH}px`;
  svgClone.style.display = 'block';
  content.appendChild(svgClone);

  // 底部工具栏
  const toolbar = document.createElement('div');
  toolbar.className = 'mermaid-overlay-toolbar';

  const zoomOut = document.createElement('button');
  zoomOut.className = 'mermaid-overlay-btn';
  zoomOut.type = 'button';
  zoomOut.setAttribute('aria-label', '缩小');
  zoomOut.textContent = '−';

  const zoomReset = document.createElement('button');
  zoomReset.className = 'mermaid-overlay-btn';
  zoomReset.type = 'button';
  zoomReset.setAttribute('aria-label', '重置');
  zoomReset.textContent = '⤢';

  const zoomIn = document.createElement('button');
  zoomIn.className = 'mermaid-overlay-btn';
  zoomIn.type = 'button';
  zoomIn.setAttribute('aria-label', '放大');
  zoomIn.textContent = '+';

  zoomIn.addEventListener('click', (e) => {
    e.stopPropagation();
    zoomAt(window.innerWidth / 2, window.innerHeight / 2, scale * 1.3);
  });
  zoomOut.addEventListener('click', (e) => {
    e.stopPropagation();
    zoomAt(window.innerWidth / 2, window.innerHeight / 2, scale / 1.3);
  });
  zoomReset.addEventListener('click', (e) => {
    e.stopPropagation();
    fitToScreen();
  });

  toolbar.append(zoomOut, zoomReset, zoomIn);

  // 底部提示
  const hint = document.createElement('div');
  hint.className = 'mermaid-overlay-hint';
  hint.textContent = '拖拽移动 · 滚轮缩放 · ESC 关闭';

  overlay.append(closeBtn, content, toolbar, hint);
  document.body.appendChild(overlay);

  // 锁定 body 滚动
  prevBodyOverflow = document.body.style.overflow;
  document.body.style.overflow = 'hidden';

  // 初始适配
  fitToScreen();

  // 绑定交互
  bindInteractions(closeBtn);
}

// ── 交互绑定 ──

function bindInteractions(closeBtn: HTMLButtonElement): void {
  if (!overlay || !content) return;

  // 关闭：✕ 按钮（stopPropagation 防止穿透到背景）
  closeBtn.addEventListener('click', closeOverlay);

  // 关闭：点击背景（非内容区）
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) closeOverlay();
  });

  // ESC 关闭
  const onKey = (e: KeyboardEvent): void => {
    if (e.key === 'Escape' && overlay) {
      e.preventDefault();
      closeOverlay();
    }
  };
  document.addEventListener('keydown', onKey, { once: true });

  // 双击：切换适配 / 原始大小
  content.addEventListener('dblclick', (e) => {
    const px = e.clientX;
    const py = e.clientY;
    if (scale < 0.99) {
      // 当前是适配模式 → 放大到 100%（原始尺寸），锚定双击点
      zoomAt(px, py, 1);
    } else {
      fitToScreen();
    }
  });

  // Pointer Events：统一鼠标 + 触摸
  content.addEventListener('pointerdown', onPointerDown);
  content.addEventListener('pointermove', onPointerMove);
  content.addEventListener('pointerup', onPointerUp);
  content.addEventListener('pointercancel', onPointerUp);
  // 阻止浏览器默认触摸手势（滚动 / 捏合缩放页面）
  content.style.touchAction = 'none';

  // 滚轮缩放
  content.addEventListener('wheel', onWheel, { passive: false });

  // 窗口尺寸变化时重新适配
  window.addEventListener('resize', onResize);
}

function onPointerDown(e: PointerEvent): void {
  if (!content) return;
  content.setPointerCapture(e.pointerId);
  pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });

  if (pointers.size === 1) {
    panStart = { x: e.clientX, y: e.clientY, tx, ty };
  } else if (pointers.size === 2) {
    panStart = null; // 切到 pinch 模式
    const pts = [...pointers.values()];
    pinchStart = {
      dist: Math.hypot(pts[1].x - pts[0].x, pts[1].y - pts[0].y),
      cx: (pts[0].x + pts[1].x) / 2,
      cy: (pts[0].y + pts[1].y) / 2,
      scale,
      tx,
      ty,
    };
  }
}

function onPointerMove(e: PointerEvent): void {
  if (!pointers.has(e.pointerId)) return;
  pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });

  if (pointers.size === 1 && panStart) {
    // 单指拖拽 = 平移
    tx = panStart.tx + (e.clientX - panStart.x);
    ty = panStart.ty + (e.clientY - panStart.y);
    applyTransform();
  } else if (pointers.size >= 2 && pinchStart) {
    // 双指 = 捏合缩放（以两指中心为锚点）
    const pts = [...pointers.values()];
    const curDist = Math.hypot(pts[1].x - pts[0].x, pts[1].y - pts[0].y);
    const newScale = (pinchStart.scale * curDist) / pinchStart.dist;
    // 锚定初始捏合中心
    const cx = (pinchStart.cx - originX - pinchStart.tx) / pinchStart.scale;
    const cy = (pinchStart.cy - originY - pinchStart.ty) / pinchStart.scale;
    const clamped = clamp(newScale, MIN_SCALE, MAX_SCALE);
    tx = pinchStart.cx - originX - clamped * cx;
    ty = pinchStart.cy - originY - clamped * cy;
    scale = clamped;
    applyTransform();
  }
}

function onPointerUp(e: PointerEvent): void {
  pointers.delete(e.pointerId);
  if (pointers.size < 2) pinchStart = null;
  if (pointers.size === 1) {
    // 从 pinch 回落到单指，重新设 pan 起点
    const pt = [...pointers.values()][0];
    panStart = { x: pt.x, y: pt.y, tx, ty };
  }
  if (pointers.size === 0) panStart = null;
}

function onWheel(e: WheelEvent): void {
  e.preventDefault();
  const factor = e.deltaY > 0 ? 1 / 1.15 : 1.15;
  zoomAt(e.clientX, e.clientY, scale * factor);
}

function onResize(): void {
  if (overlay) fitToScreen();
}

// ── 开/关浮层 ──

/** 同步关闭浮层并清理全部状态（无动画延迟）。 */
function destroyOverlay(): void {
  if (overlay) overlay.remove();
  overlay = null;
  content = null;
  svgClone = null;
  pointers.clear();
  panStart = null;
  pinchStart = null;
  document.body.style.overflow = prevBodyOverflow;
  window.removeEventListener('resize', onResize);
}

/**
 * 关闭浮层（带淡出动画）。
 * 动画结束后 CSS 自然移除元素；状态立即清空避免重复打开。
 */
function closeOverlay(): void {
  if (!overlay) return;
  overlay.classList.add('mermaid-overlay-closing');
  const el = overlay;
  // 动画结束后移除；fallback 兜底防 animationend 不触发
  el.addEventListener('animationend', () => el.remove(), { once: true });
  setTimeout(() => el.remove(), 250);

  // 立即清理状态
  overlay = null;
  content = null;
  svgClone = null;
  pointers.clear();
  panStart = null;
  pinchStart = null;
  document.body.style.overflow = prevBodyOverflow;
  window.removeEventListener('resize', onResize);
}

// ── 对外接口 ──

/**
 * 为已渲染的 mermaid `<pre>` 绑定点击放大。
 *
 * 在 renderBlock 成功后调用。仅绑定一次（dataset 守卫），主题切换重渲染
 * 不需重新绑定——click handler 在 `<pre>` 上，重渲染只替换 innerHTML。
 */
export function attachOverlayTrigger(pre: HTMLPreElement): void {
  if (pre.dataset.mermaidOverlayReady) return;
  pre.dataset.mermaidOverlayReady = 'true';

  pre.addEventListener('click', () => {
    if (overlay) return; // 已有浮层打开，忽略
    const svg = pre.querySelector('svg');
    if (!svg) return;
    buildOverlay(svg);
  });
}

/** 测试用：同步重置浮层状态（移除 DOM + 清空单例变量）。 */
export function _resetOverlay(): void {
  destroyOverlay();
}

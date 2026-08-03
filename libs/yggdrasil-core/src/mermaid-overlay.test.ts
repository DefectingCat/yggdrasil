/**
 * mermaid-overlay 测试：钉住与图片灯箱统一的交互契约。
 *
 * 覆盖：
 * - flyStateFor 纯函数数学（SVG 屏幕 rect → 浮层 transform 态）
 * - 打开 FLIP 飞行（首帧在原位、double-rAF 后飞到居中 fit 态）
 * - 关闭飞回（Esc/✕ → transition + 飞回 transform，动画后移除）
 * - Esc 监听持久性回归（按过其他键后 Esc 仍生效）
 * - 滚动驱动关闭（部分滚动插值 + 淡出、滚回还原、滚满 120px 立即移除）
 * - 滚轮缩放收敛为 Ctrl/⌘+滚轮（普通滚轮不拦截、不缩放）
 * - reduced-motion（打开无飞行、滚动立即关闭、关闭无动画）
 *
 * 驱动方式：直接 attachOverlayTrigger 到手写的 <pre><svg>，不经 initMermaid
 * （渲染管线的黑盒测试在 mermaid.test.ts）。happy-dom 的 matchMedia 默认
 * matches=false（非 reduced），innerWidth/innerHeight 默认 1024×768。
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { _resetOverlay, attachOverlayTrigger, flyStateFor } from './mermaid-overlay';

// happy-dom 默认视口 1024×768；viewBox 400×200 → fitScale = min(976/400, 720/200, 1) = 1，
// originX = (1024-400)/2 = 312，originY = (768-200)/2 = 284。
const ORIGIN_X = 312;
const ORIGIN_Y = 284;

/** SVG 在文章里的屏幕 rect（飞行起点/终点）。 */
const SVG_RECT = {
  left: 10,
  top: 20,
  width: 200,
  height: 100,
  right: 210,
  bottom: 120,
  x: 10,
  y: 20,
  toJSON: () => ({}),
} as DOMRect;

/** 原位态：scale = 200/400 = 0.5，tx = 10-312 = -302，ty = 20-284 = -264。 */
const FLY_TRANSFORM = 'translate(-302px, -264px) scale(0.5)';
/** fit 态：tx/ty 恒为 0，scale = fitScale = 1。 */
const FIT_TRANSFORM = 'translate(0px, 0px) scale(1)';

/** 造一个已渲染的 mermaid <pre>（含 viewBox 400×200 的 SVG），绑定点击放大。 */
function makePre(): HTMLPreElement {
  const pre = document.createElement('pre');
  pre.innerHTML = '<svg viewBox="0 0 400 200"><rect width="400" height="200"/></svg>';
  document.body.appendChild(pre);
  const svg = pre.querySelector('svg');
  if (!svg) throw new Error('测试夹具缺 svg');
  vi.spyOn(svg, 'getBoundingClientRect').mockReturnValue(SVG_RECT);
  attachOverlayTrigger(pre);
  return pre;
}

function contentEl(): HTMLDivElement {
  const el = document.querySelector('.mermaid-overlay-content');
  if (!(el instanceof HTMLDivElement)) throw new Error('浮层 content 不存在');
  return el;
}

/** 等 double-rAF 打开动画的目标帧生效（transform 到达 fit 态）。 */
async function waitFitTransform(): Promise<void> {
  await vi.waitFor(() => {
    expect(contentEl().style.transform).toBe(FIT_TRANSFORM);
  });
}

/** 临时把 window.scrollY 钉到指定值（happy-dom 默认 0）。 */
function stubScrollY(value: number): void {
  Object.defineProperty(window, 'scrollY', { value, configurable: true });
}

function mockReducedMotion(matches: boolean): void {
  vi.spyOn(window, 'matchMedia').mockReturnValue({
    matches,
    media: '(prefers-reduced-motion: reduce)',
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    onchange: null,
    dispatchEvent: () => false,
  } as MediaQueryList);
}

describe('flyStateFor', () => {
  it('把 SVG 屏幕 rect 映射成浮层 transform 态', () => {
    const fly = flyStateFor({ x: 10, y: 20, w: 200, h: 100 }, ORIGIN_X, ORIGIN_Y, 400);
    expect(fly).toEqual({ scale: 0.5, tx: -302, ty: -264 });
  });

  it('naturalW 为 0 时退化为 scale 1', () => {
    const fly = flyStateFor({ x: 10, y: 20, w: 200, h: 100 }, ORIGIN_X, ORIGIN_Y, 0);
    expect(fly.scale).toBe(1);
  });
});

describe('mermaid-overlay 灯箱统一交互', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });
  afterEach(() => {
    _resetOverlay();
    document.body.innerHTML = '';
    vi.restoreAllMocks();
    stubScrollY(0);
  });

  it('打开：首帧在 SVG 原位（无 transition、透明）', () => {
    const pre = makePre();
    pre.click();
    const content = contentEl();
    expect(content.style.transition).toBe('none');
    expect(content.style.transform).toBe(FLY_TRANSFORM);
    expect(content.style.opacity).toBe('0');
  });

  it('打开：double-rAF 后带 transition 飞到居中 fit 态', async () => {
    const pre = makePre();
    pre.click();
    await waitFitTransform();
    const content = contentEl();
    expect(content.style.transition).toContain('transform 250ms ease-out');
    expect(content.style.opacity).toBe('1');
  });

  it('Esc 关闭：内容带 transition 飞回 SVG 原位，动画后移除浮层', async () => {
    const pre = makePre();
    pre.click();
    await waitFitTransform();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));

    // 飞回帧立即生效
    const content = contentEl();
    expect(content.style.transition).toContain('transform 250ms ease-out');
    expect(content.style.transform).toBe(FLY_TRANSFORM);
    expect(content.style.opacity).toBe('0');
    expect(
      document.querySelector('.mermaid-overlay')?.classList.contains('mermaid-overlay-closing'),
    ).toBe(true);
    // happy-dom 不跑 CSS transition → 280ms 兜底定时器移除
    await vi.waitFor(() => {
      expect(document.querySelector('.mermaid-overlay')).toBeNull();
    });
  });

  it('回归：按过其他键后 Esc 仍能关闭（持久监听，非 once）', async () => {
    const pre = makePre();
    pre.click();
    await waitFitTransform();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'a' }));
    expect(document.querySelector('.mermaid-overlay')).not.toBeNull();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await vi.waitFor(() => {
      expect(document.querySelector('.mermaid-overlay')).toBeNull();
    });
  });

  it('滚动驱动关闭：部分滚动按进度插值 transform 与透明度', async () => {
    const pre = makePre();
    pre.click();
    await waitFitTransform();

    // 滚 60px → progress = 0.5：scale = 1+(0.5-1)*0.5 = 0.75，
    // tx = -302*0.5 = -151，ty = -264*0.5 = -132，opacity 0.5
    stubScrollY(60);
    window.dispatchEvent(new Event('scroll'));

    const content = contentEl();
    expect(content.style.transition).toBe('none');
    expect(content.style.transform).toBe('translate(-151px, -132px) scale(0.75)');
    expect(content.style.opacity).toBe('0.5');
    const overlay = document.querySelector('.mermaid-overlay') as HTMLDivElement;
    expect(overlay.style.opacity).toBe('0.5');
  });

  it('滚动驱动关闭：滚回原位还原 fit 态', async () => {
    const pre = makePre();
    pre.click();
    await waitFitTransform();

    stubScrollY(60);
    window.dispatchEvent(new Event('scroll'));
    stubScrollY(0);
    window.dispatchEvent(new Event('scroll'));

    const content = contentEl();
    expect(content.style.transform).toBe(FIT_TRANSFORM);
    expect(content.style.opacity).toBe('1');
    expect(document.querySelector('.mermaid-overlay')).not.toBeNull();
  });

  it('滚动驱动关闭：滚满 120px 立即同步移除浮层', async () => {
    const pre = makePre();
    pre.click();
    await waitFitTransform();

    stubScrollY(130);
    window.dispatchEvent(new Event('scroll'));

    expect(document.querySelector('.mermaid-overlay')).toBeNull();
  });

  it('普通滚轮不缩放也不拦截（留给滚动关闭）', async () => {
    const pre = makePre();
    pre.click();
    await waitFitTransform();

    const ev = new WheelEvent('wheel', { deltaY: -100, bubbles: true, cancelable: true });
    contentEl().dispatchEvent(ev);

    expect(ev.defaultPrevented).toBe(false);
    expect(contentEl().style.transform).toBe(FIT_TRANSFORM);
  });

  it('Ctrl+滚轮缩放（触控板捏合）', async () => {
    const pre = makePre();
    pre.click();
    await waitFitTransform();

    // happy-dom 的 WheelEvent 不从 init dict 读 ctrlKey（实证为 undefined），手动钉上。
    const ev = new WheelEvent('wheel', {
      deltaY: -100,
      bubbles: true,
      cancelable: true,
    });
    Object.defineProperty(ev, 'ctrlKey', { value: true });
    contentEl().dispatchEvent(ev);

    expect(ev.defaultPrevented).toBe(true);
    expect(contentEl().style.transform).toContain('scale(1.15)');
  });

  it('宽图（fitScale<1）fit 态居中：origin 乘 scale，不偏左/偏高', async () => {
    // viewBox 2000×500，fitScale = min(976/2000, 720/500, 1) = 0.488。
    // 正确居中：originX = (1024 − 2000×0.488)/2 = 24，originY = (768 − 500×0.488)/2 = 262。
    // 回归：旧公式用未缩放尺寸得 originX = −488，宽图左侧被裁出视口。
    const pre = document.createElement('pre');
    pre.innerHTML = '<svg viewBox="0 0 2000 500"><rect width="2000" height="500"/></svg>';
    document.body.appendChild(pre);
    const svg = pre.querySelector('svg');
    if (!svg) throw new Error('测试夹具缺 svg');
    vi.spyOn(svg, 'getBoundingClientRect').mockReturnValue(SVG_RECT);
    attachOverlayTrigger(pre);

    pre.click();
    await vi.waitFor(() => {
      expect(contentEl().style.transform).toBe('translate(0px, 0px) scale(0.488)');
    });
    const content = contentEl();
    expect(content.style.left).toBe('24px');
    expect(content.style.top).toBe('262px');
  });

  it('工具栏 + 按钮：缩放带 200ms 过渡，结束后清回 none', async () => {
    const pre = makePre();
    pre.click();
    await waitFitTransform();

    const zoomIn = document.querySelectorAll('.mermaid-overlay-btn')[2] as HTMLButtonElement;
    zoomIn.click();

    // 离散指令：带 transform 过渡，目标 scale = 1×1.3
    const content = contentEl();
    expect(content.style.transition).toContain('transform 200ms ease-out');
    expect(content.style.transform).toContain('scale(1.3)');

    // 过渡结束（happy-dom 无 CSS transition，走 230ms 兜底）后清回 none，
    // 交还拖拽/滚轮的即时响应
    await vi.waitFor(() => {
      expect(content.style.transition).toBe('none');
    });
  });

  it('双击：带过渡放大到 100% 原始尺寸', async () => {
    // 用宽图让 fitScale<1（0.488），双击放大到 1
    const pre = document.createElement('pre');
    pre.innerHTML = '<svg viewBox="0 0 2000 500"><rect width="2000" height="500"/></svg>';
    document.body.appendChild(pre);
    const svg = pre.querySelector('svg');
    if (!svg) throw new Error('测试夹具缺 svg');
    vi.spyOn(svg, 'getBoundingClientRect').mockReturnValue(SVG_RECT);
    attachOverlayTrigger(pre);

    pre.click();
    await vi.waitFor(() => {
      expect(contentEl().style.transform).toBe('translate(0px, 0px) scale(0.488)');
    });

    contentEl().dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    const content = contentEl();
    expect(content.style.transition).toContain('transform 200ms ease-out');
    expect(content.style.transform).toContain('scale(1)');
  });

  it('reduced-motion：工具栏缩放即时生效无过渡', () => {
    mockReducedMotion(true);
    const pre = makePre();
    pre.click();
    const zoomIn = document.querySelectorAll('.mermaid-overlay-btn')[2] as HTMLButtonElement;
    zoomIn.click();
    const content = contentEl();
    expect(content.style.transform).toContain('scale(1.3)');
    expect(content.style.transition).not.toContain('transform');
  });

  it('reduced-motion：打开无飞行（内容直接 fit 态，无 transform transition）', () => {
    mockReducedMotion(true);
    const pre = makePre();
    pre.click();
    const content = contentEl();
    expect(content.style.transform).toBe(FIT_TRANSFORM);
    expect(content.style.transition).not.toContain('transform');
  });

  it('reduced-motion：滚动立即关闭', () => {
    mockReducedMotion(true);
    const pre = makePre();
    pre.click();
    expect(document.querySelector('.mermaid-overlay')).not.toBeNull();

    stubScrollY(10);
    window.dispatchEvent(new Event('scroll'));

    expect(document.querySelector('.mermaid-overlay')).toBeNull();
  });

  it('reduced-motion：Esc 同步关闭（无飞回动画）', () => {
    mockReducedMotion(true);
    const pre = makePre();
    pre.click();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(document.querySelector('.mermaid-overlay')).toBeNull();
  });

  it('关闭后焦点归还 <pre>', async () => {
    const pre = makePre();
    pre.click();
    await waitFitTransform();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await vi.waitFor(() => {
      expect(document.querySelector('.mermaid-overlay')).toBeNull();
    });
    expect(document.activeElement).toBe(pre);
  });
});

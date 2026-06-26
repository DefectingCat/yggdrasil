/**
 * theme-transition 测试。
 *
 * happy-dom 不提供 document.startViewTransition,天然覆盖降级路径(瞬切 dark class)。
 * 主路径通过 mock startViewTransition 验证:调用它、设 CSS 变量。
 *
 * 注意:startThemeTransition 只接收 (x, y),目标主题(亮/暗)从 DOM 的 dark class
 * 现状推导(取反),不依赖外部传入——避免与调用方状态不同步。
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import './index';

describe('startThemeTransition', () => {
  beforeEach(() => {
    document.documentElement.classList.remove('dark');
    document.documentElement.classList.remove('is-theme-transitioning');
    document.documentElement.style.cssText = '';
    vi.restoreAllMocks();
  });
  afterEach(() => {
    document.documentElement.classList.remove('dark');
    document.documentElement.classList.remove('is-theme-transitioning');
    document.documentElement.style.cssText = '';
  });

  it('降级:无 startViewTransition 时,亮→暗(无 dark class 时 add)', () => {
    expect(document.documentElement.classList.contains('dark')).toBe(false);

    window.__startThemeTransition(100, 200);

    expect(document.documentElement.classList.contains('dark')).toBe(true);
  });

  it('降级:无 startViewTransition 时,暗→亮(有 dark class 时 remove)', () => {
    document.documentElement.classList.add('dark');

    window.__startThemeTransition(100, 200);

    expect(document.documentElement.classList.contains('dark')).toBe(false);
  });

  it('主路径:有 startViewTransition 时调用它,callback 切换 dark class', async () => {
    const cbRef: { cb: (() => void) | null } = { cb: null };
    const readyP = Promise.resolve();
    const finishedP = Promise.resolve();
    const startVT = vi.fn((cb: () => void) => {
      cbRef.cb = cb;
      return { ready: readyP, finished: finishedP, skipTransition: () => {} };
    });
    Object.defineProperty(document, 'startViewTransition', {
      value: startVT,
      configurable: true,
      writable: true,
    });

    // Mock animate on documentElement (happy-dom 不支持 pseudoElement)
    const animateSpy = vi.fn(() => ({ finished: Promise.resolve() }));
    document.documentElement.animate = animateSpy as unknown as typeof document.documentElement.animate;

    window.__startThemeTransition(100, 200);

    expect(startVT).toHaveBeenCalledTimes(1);

    // is-theme-transitioning 应在 VT 之前添加
    expect(document.documentElement.classList.contains('is-theme-transitioning')).toBe(true);

    // callback 里根据 DOM 现状(无 dark)切到 dark
    cbRef.cb!();
    expect(document.documentElement.classList.contains('dark')).toBe(true);

    // ready 后通过 Web Animations API 控制动画
    await readyP;
    await Promise.resolve();
    // animate 应被调用两次:一次 old(opacity),一次 new(clipPath + opacity)
    expect(animateSpy).toHaveBeenCalledTimes(2);
    // 第二次调用(new)应包含 clipPath
    const calls = animateSpy.mock.calls as unknown[][];
    const newCall = calls[1];
    expect(newCall[0]).toHaveProperty('clipPath');
    expect(newCall[1]).toHaveProperty('pseudoElement', '::view-transition-new(root)');

    // finished 后移除 is-theme-transitioning
    await finishedP;
    await Promise.resolve();
    // happy-dom microtask 可能需要额外 tick
    await new Promise((r) => setTimeout(r, 0));
    expect(document.documentElement.classList.contains('is-theme-transitioning')).toBe(false);

    delete (document as unknown as { startViewTransition?: unknown }).startViewTransition;
  });

  it('reduced-motion:即使有 startViewTransition 也走降级(瞬切)', () => {
    const startVT = vi.fn(() => ({ ready: Promise.resolve(), finished: Promise.resolve(), skipTransition: () => {} }));
    Object.defineProperty(document, 'startViewTransition', {
      value: startVT,
      configurable: true,
      writable: true,
    });
    vi.stubGlobal('matchMedia', vi.fn((q: string) => ({
      matches: q.includes('reduce'),
      media: q,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    })));

    window.__startThemeTransition(0, 0);

    expect(startVT).not.toHaveBeenCalled();
    expect(document.documentElement.classList.contains('dark')).toBe(true);

    delete (document as unknown as { startViewTransition?: unknown }).startViewTransition;
  });
});

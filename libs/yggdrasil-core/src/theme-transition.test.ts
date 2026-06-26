/**
 * theme-transition 测试。
 *
 * happy-dom 不提供 document.startViewTransition,天然覆盖降级路径(瞬切 dark class)。
 * 主路径通过 mock startViewTransition 验证:调用它、传对参数、设 CSS 变量。
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import './index';

describe('startThemeTransition', () => {
  beforeEach(() => {
    document.documentElement.classList.remove('dark');
    document.documentElement.style.cssText = '';
    vi.restoreAllMocks();
  });
  afterEach(() => {
    document.documentElement.classList.remove('dark');
    document.documentElement.style.cssText = '';
  });

  it('降级:无 startViewTransition 时直接 toggle dark class(切到 dark)', () => {
    expect(document.documentElement.classList.contains('dark')).toBe(false);

    window.__startThemeTransition(100, 200, true);

    expect(document.documentElement.classList.contains('dark')).toBe(true);
  });

  it('降级:切到 light 时移除 dark class', () => {
    document.documentElement.classList.add('dark');

    window.__startThemeTransition(100, 200, false);

    expect(document.documentElement.classList.contains('dark')).toBe(false);
  });

  it('降级时不设 CSS 变量(--tt-x/y/max-r 仅主路径用)', () => {
    window.__startThemeTransition(100, 200, true);

    expect(document.documentElement.style.getPropertyValue('--tt-x')).toBe('');
  });

  it('主路径:有 startViewTransition 时调用它并设 CSS 变量', () => {
    const cbRef: { cb: (() => void) | null } = { cb: null };
    const startVT = vi.fn((cb: () => void) => {
      cbRef.cb = cb;
      return { finished: Promise.resolve(), skipTransition: () => {} };
    });
    Object.defineProperty(document, 'startViewTransition', {
      value: startVT,
      configurable: true,
      writable: true,
    });

    window.__startThemeTransition(100, 200, true);

    // 调了 startViewTransition
    expect(startVT).toHaveBeenCalledTimes(1);
    // 设了圆心与半径变量
    expect(document.documentElement.style.getPropertyValue('--tt-x')).toBe('100px');
    expect(document.documentElement.style.getPropertyValue('--tt-y')).toBe('200px');
    // max-r 是个正数 px 值
    expect(document.documentElement.style.getPropertyValue('--tt-max-r')).toMatch(/^\d+(\.\d+)?px$/);

    // 回调里 toggle dark class(模拟浏览器截图前)
    expect(document.documentElement.classList.contains('dark')).toBe(false);
    cbRef.cb!();
    expect(document.documentElement.classList.contains('dark')).toBe(true);

    delete (document as unknown as { startViewTransition?: unknown }).startViewTransition;
  });

  it('reduced-motion:即使有 startViewTransition 也走降级(瞬切)', () => {
    const startVT = vi.fn(() => ({ finished: Promise.resolve(), skipTransition: () => {} }));
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

    window.__startThemeTransition(0, 0, true);

    expect(startVT).not.toHaveBeenCalled();
    expect(document.documentElement.classList.contains('dark')).toBe(true);

    delete (document as unknown as { startViewTransition?: unknown }).startViewTransition;
  });
});

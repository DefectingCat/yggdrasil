/**
 * hash-scroll 测试:钉住锚点跳转的解码与手动偏移滚动行为。
 * 黑盒驱动:只通过 window.__scrollToHash 入口。
 *
 * 现在用 window.scrollTo 手动扣除 sticky header 高度（不再依赖 scrollIntoView +
 * scroll-margin-top，因为浏览器原生 fragment-scroll 对后者的应用时序不稳定）。
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import './index';

describe('scrollToHash', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    history.replaceState(null, '', '#');
    vi.restoreAllMocks();
  });
  afterEach(() => {
    document.body.innerHTML = '';
    history.replaceState(null, '', '#');
  });

  it('hash 为 CJK 百分号编码时解码后命中并滚动', () => {
    const heading = document.createElement('h2');
    heading.id = '三-五-零法则';
    document.body.appendChild(heading);

    const spy = vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
    history.replaceState(null, '', `#${encodeURIComponent(heading.id)}`);

    window.__scrollToHash();

    expect(spy).toHaveBeenCalled();
  });

  it('hash 为纯 ASCII 时直接命中', () => {
    const heading = document.createElement('h2');
    heading.id = 'getting-started';
    document.body.appendChild(heading);

    const spy = vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
    history.replaceState(null, '', '#getting-started');

    window.__scrollToHash();

    expect(spy).toHaveBeenCalled();
  });

  it('滚动落点扣除 sticky header 高度', () => {
    // 模拟 sticky header：getBoundingClientRect 返回 height 80。
    const header = document.createElement('header');
    header.className = 'sticky';
    header.getBoundingClientRect = () =>
      ({
        top: 0,
        left: 0,
        right: 0,
        bottom: 80,
        width: 1000,
        height: 80,
        x: 0,
        y: 0,
        toJSON() {},
      }) as DOMRect;
    document.body.appendChild(header);

    // 目标标题：放在 scrollY=1000 处（getBoundingClientRect.top 反映相对视口位置）。
    const heading = document.createElement('h2');
    heading.id = 'mid';
    heading.getBoundingClientRect = () =>
      ({
        top: 1000,
        left: 0,
        right: 0,
        bottom: 1050,
        width: 800,
        height: 50,
        x: 0,
        y: 1000,
        toJSON() {},
      }) as DOMRect;
    document.body.appendChild(heading);

    // scrollY 参与 scrollTo 目标计算：top = rect.top + scrollY - headerH - offset。
    Object.defineProperty(window, 'scrollY', { value: 500, writable: true, configurable: true });

    const spy = vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
    history.replaceState(null, '', '#mid');

    window.__scrollToHash();

    // 第一帧调用（smooth）：top = 1000 + 500 - 80 - 16 = 1404
    expect(spy).toHaveBeenCalled();
    const firstCall = spy.mock.calls[0][0] as ScrollToOptions;
    expect(firstCall.top).toBe(1000 + 500 - 80 - 16);
    expect(firstCall.behavior).toBe('smooth');
  });

  it('hash 为空时不滚动也不报错', () => {
    const spy = vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
    history.replaceState(null, '', '#');
    expect(() => window.__scrollToHash()).not.toThrow();
    expect(spy).not.toHaveBeenCalled();
  });

  it('目标元素不存在时不报错', () => {
    const spy = vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
    history.replaceState(null, '', '#no-such-heading');
    expect(() => window.__scrollToHash()).not.toThrow();
    expect(spy).not.toHaveBeenCalled();
  });
});

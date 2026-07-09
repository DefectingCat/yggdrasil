/**
 * hash-scroll 测试:钉住锚点跳转的解码与 scrollIntoView 行为。
 * 黑盒驱动:只通过 window.__scrollToHash 入口。
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
    // 标题 id 是原始 CJK 字符，URL hash 是百分号编码形式。
    const heading = document.createElement('h2');
    heading.id = '三-五-零法则';
    document.body.appendChild(heading);

    const spy = vi.spyOn(heading, 'scrollIntoView');
    history.replaceState(null, '', `#${encodeURIComponent(heading.id)}`);

    window.__scrollToHash();

    expect(spy).toHaveBeenCalledOnce();
    expect(spy).toHaveBeenCalledWith({ behavior: 'smooth', block: 'start' });
  });

  it('hash 为纯 ASCII 时直接命中', () => {
    const heading = document.createElement('h2');
    heading.id = 'getting-started';
    document.body.appendChild(heading);

    const spy = vi.spyOn(heading, 'scrollIntoView');
    history.replaceState(null, '', '#getting-started');

    window.__scrollToHash();

    expect(spy).toHaveBeenCalledOnce();
  });

  it('hash 为空时不滚动也不报错', () => {
    history.replaceState(null, '', '#');
    expect(() => window.__scrollToHash()).not.toThrow();
  });

  it('目标元素不存在时不报错', () => {
    history.replaceState(null, '', '#no-such-heading');
    expect(() => window.__scrollToHash()).not.toThrow();
  });
});

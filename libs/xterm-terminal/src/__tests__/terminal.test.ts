import { describe, expect, it } from 'vitest';
import { TerminalInstance, XtermOptions } from '../terminal';

describe('XtermOptions', () => {
  it('可无参构造，字段全部 undefined', () => {
    const opts = new XtermOptions();
    expect(opts.theme).toBeUndefined();
    expect(opts.fontFamily).toBeUndefined();
    expect(opts.fontSize).toBeUndefined();
    expect(opts.onReady).toBeUndefined();
  });

  it('可设置字段', () => {
    const opts = new XtermOptions();
    opts.theme = 'dark';
    opts.fontSize = 14;
    expect(opts.theme).toBe('dark');
    expect(opts.fontSize).toBe(14);
  });
});

describe('TerminalInstance', () => {
  it('挂载到容器并触发 onReady', () => {
    const container = document.createElement('div');
    let ready = false;
    const opts = new XtermOptions();
    opts.onReady = () => {
      ready = true;
    };

    const inst = new TerminalInstance(container, opts);
    expect(ready).toBe(true);

    // xterm.js 在容器内创建 .xterm 元素
    expect(container.querySelector('.xterm')).toBeTruthy();

    inst.destroy();
  });

  it('writeAll 清屏后重写 stdout + stderr', () => {
    const container = document.createElement('div');
    const inst = new TerminalInstance(container, new XtermOptions());

    // 不报错即通过（xterm.js 在 happy-dom 下 write 是 no-op 渲染）
    inst.writeAll('hello\n', 'error\n');
    inst.clear();

    inst.destroy();
  });
});

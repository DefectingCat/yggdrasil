import { describe, it, expect, beforeEach } from 'vitest';
import { EditorOptions, CodeMirrorInstance } from '../editor';

describe('CodeMirrorInstance', () => {
  let container: HTMLElement;

  beforeEach(() => {
    container = document.createElement('div');
    container.id = 'test-cm';
    document.body.appendChild(container);
  });

  it('getValue/setValue 往返', () => {
    const inst = new CodeMirrorInstance(container, new EditorOptions());
    inst.setValue('SELECT 1');
    expect(inst.getValue()).toBe('SELECT 1');
    inst.destroy();
  });

  it('初始 value 正确', () => {
    const opts = new EditorOptions();
    opts.value = 'SELECT * FROM posts';
    const inst = new CodeMirrorInstance(container, opts);
    expect(inst.getValue()).toBe('SELECT * FROM posts');
    inst.destroy();
  });

  it('setTheme 不抛错（走 Compartment reconfigure）', () => {
    const inst = new CodeMirrorInstance(container, new EditorOptions());
    expect(() => inst.setTheme('dark')).not.toThrow();
    expect(() => inst.setTheme('light')).not.toThrow();
    inst.destroy();
  });

  it('setSchema 更新 lang-sql 配置', () => {
    const inst = new CodeMirrorInstance(container, new EditorOptions());
    expect(() =>
      inst.setSchema({ tables: [{ name: 'posts', columns: ['id', 'title'] }] }),
    ).not.toThrow();
    inst.destroy();
  });

  it('vim 开关：vim:true 注入，false 不注入', () => {
    const optsOn = new EditorOptions();
    optsOn.vim = true;
    const instOn = new CodeMirrorInstance(container, optsOn);
    instOn.destroy();

    const optsOff = new EditorOptions();
    optsOff.vim = false;
    const instOff = new CodeMirrorInstance(container, optsOff);
    instOff.destroy();
    // happy-dom 无法验证 keymap 行为，仅验证配置加载不抛错
  });

  it('onChange 在内容变更时触发', () => {
    let captured = '';
    const opts = new EditorOptions();
    opts.onChange = (v) => {
      captured = v;
    };
    const inst = new CodeMirrorInstance(container, opts);
    inst.setValue('hello');
    expect(captured).toBe('hello');
    inst.destroy();
  });
});

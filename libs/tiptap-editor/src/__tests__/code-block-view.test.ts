import { describe, expect, it, vi } from 'vitest';
import { CodeBlockNodeView } from '../code-block-view';

/**
 * 模块级共享 sentinel：对齐 upload-image.test.ts 范式。
 * 真实 ProseMirror 每个 schema 的 NodeType 是单例，同类节点引用必等；
 * mock 用共享引用模拟此语义，使实现的纯引用比较能命中。
 */
const CODEBLOCK_TYPE = { name: 'codeBlock' };
const PARAGRAPH_TYPE = { name: 'paragraph' };

/** 构造最小 mock node，只含 NodeView 需要的字段。 */
function mockNode(language: string, textContent = '') {
  return {
    type: CODEBLOCK_TYPE,
    attrs: { language },
    textContent,
  } as any;
}

/** 构造最小 mock editor，含 storage。 */
function mockEditor(onRunCode?: (opts: any) => Promise<string>) {
  return {
    storage: onRunCode ? { __onRunCode: onRunCode } : {},
  } as any;
}

/**
 * CodeBlockNodeView 测试。
 *
 * 验证：DOM 结构、语言标签、运行按钮显隐、update 刷新、contentDOM 指向 code。
 */
describe('CodeBlockNodeView', () => {
  it('runnable 块：toolbar 显示语言 + 运行按钮', () => {
    const view = new CodeBlockNodeView({
      node: mockNode('python runnable {"timeout_secs":10}'),
      editor: mockEditor(),
    } as any);
    expect(view.dom.querySelector('.tiptap-codeblock-lang')?.textContent).toBe('python');
    expect(view.dom.querySelector('.tiptap-codeblock-run')).not.toBeNull();
  });

  it('普通块(python)：显示语言标签，无运行按钮', () => {
    const view = new CodeBlockNodeView({
      node: mockNode('python'),
      editor: mockEditor(),
    } as any);
    expect(view.dom.querySelector('.tiptap-codeblock-lang')?.textContent).toBe('python');
    expect(view.dom.querySelector('.tiptap-codeblock-run')).toBeNull();
  });

  it('无 language 的块：标签为空或占位，无运行按钮', () => {
    const view = new CodeBlockNodeView({
      node: mockNode(''),
      editor: mockEditor(),
    } as any);
    expect(view.dom.querySelector('.tiptap-codeblock-run')).toBeNull();
  });

  it('node 语言变化时 update() 刷新标签', () => {
    const view = new CodeBlockNodeView({
      node: mockNode('python'),
      editor: mockEditor(),
    } as any);
    // 模拟 ProseMirror 调 update：改为 node runnable
    const updated = view.update(mockNode('node runnable'));
    expect(updated).toBe(true);
    expect(view.dom.querySelector('.tiptap-codeblock-lang')?.textContent).toBe('node');
    expect(view.dom.querySelector('.tiptap-codeblock-run')).not.toBeNull();
  });

  it('update 拒绝非同类节点(返回 false)', () => {
    const view = new CodeBlockNodeView({
      node: mockNode('python'),
      editor: mockEditor(),
    } as any);
    const otherNode = { type: PARAGRAPH_TYPE, attrs: {}, textContent: '' } as any;
    expect(view.update(otherNode)).toBe(false);
  });

  it('contentDOM 指向 code 元素(保证 decoration 生效)', () => {
    const view = new CodeBlockNodeView({
      node: mockNode('python'),
      editor: mockEditor(),
    } as any);
    expect(view.contentDOM?.tagName).toBe('CODE');
  });

  it('ignoreMutation 返回 true(工具栏不触发编辑事务)', () => {
    const view = new CodeBlockNodeView({
      node: mockNode('python'),
      editor: mockEditor(),
    } as any);
    expect(view.ignoreMutation()).toBe(true);
  });

  it('点击运行按钮调用 onRunCode(storage 回调)', () => {
    const onRunCode = vi.fn(() => Promise.resolve('结果'));
    const view = new CodeBlockNodeView({
      node: mockNode('python runnable', 'print(1)'),
      editor: mockEditor(onRunCode),
    } as any);
    view.dom.querySelector<HTMLButtonElement>('.tiptap-codeblock-run')!.click();
    expect(onRunCode).toHaveBeenCalledTimes(1);
    expect(onRunCode).toHaveBeenCalledWith(
      expect.objectContaining({ language: 'python runnable', source: 'print(1)' }),
    );
  });

  it('无 onRunCode 时点运行不报错(优雅降级)', () => {
    const view = new CodeBlockNodeView({
      node: mockNode('python runnable', 'print(1)'),
      editor: mockEditor(), // 无 __onRunCode
    } as any);
    expect(() =>
      view.dom.querySelector<HTMLButtonElement>('.tiptap-codeblock-run')!.click(),
    ).not.toThrow();
  });
});

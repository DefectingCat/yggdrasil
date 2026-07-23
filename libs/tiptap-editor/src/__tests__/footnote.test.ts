// @vitest-environment happy-dom

import { Editor } from '@tiptap/core';
import { Markdown } from '@tiptap/markdown';
import StarterKit from '@tiptap/starter-kit';
import { beforeEach, describe, expect, it } from 'vitest';

/**
 * 脚注语法回归测试。
 *
 * @tiptap/markdown 的 escapeMarkdownSyntax 将 `[`→`\[`、`]`→`\]`，
 * 导致 `[^1]` 被序列化为 `\[^1\]`，pulldown-cmark 将其当作字面文本。
 * TiptapEditorInstance.unescapeFootnoteSyntax 在序列化后做后处理还原。
 *
 * 此测试直接验证 escapeMarkdownSyntax 的行为以及 unescape 正则的修复效果。
 */

const UNESCAPE_RE = /\\\[\^([^\n\\]*?)\\\]/g;
const unescape = (md: string) => md.replace(UNESCAPE_RE, '[^$1]');

function makeEditor() {
  return new Editor({
    element: document.body,
    extensions: [
      StarterKit.configure({ heading: { levels: [1, 2, 3] } }),
      Markdown,
    ],
    content: '',
  });
}

describe('脚注语法 - escapeMarkdownSyntax 破坏与修复', () => {
  let editor: Editor;

  beforeEach(() => {
    editor = makeEditor();
  });

  it('escapeMarkdownSyntax 确实会把 [^1] 转义为 \\[^1\\]', () => {
    editor.commands.setContent('引用[^1]', { contentType: 'markdown' });
    const raw = editor.getMarkdown();
    // 未修复时 raw 为 `\[^1\]`——这就是脚注失效的直接原因
    expect(raw).toContain('\\[^1\\]');
  });

  it('unescape 正则还原脚注引用 \\[^1\\] → [^1]', () => {
    expect(unescape('\\[^1\\]')).toBe('[^1]');
  });

  it('unescape 正则还原含空格的脚注 label', () => {
    expect(unescape('\\[^my note\\]')).toBe('[^my note]');
  });

  it('unescape 正则还原脚注定义 \\[^1\\]: → [^1]:', () => {
    expect(unescape('\\[^1\\]: 定义内容')).toBe('[^1]: 定义内容');
  });

  it('unescape 正则不影响普通链接 [text](url)', () => {
    const md = '\\[text\\](https://example.com)';
    // 普通链接的 \[text\] 不以 ^ 开头，不会被误改
    expect(unescape(md)).toBe(md);
  });

  it('unescape 正则一次处理多个脚注引用', () => {
    const md = '引用了\\[^1\\]和\\[^pc\\]和\\[^2\\]';
    expect(unescape(md)).toBe('引用了[^1]和[^pc]和[^2]');
  });

  it('完整往返：unescape(getMarkdown()) 保留脚注引用语法', () => {
    editor.commands.setContent('正文[^1]', { contentType: 'markdown' });
    const fixed = unescape(editor.getMarkdown());
    expect(fixed).toContain('[^1]');
    expect(fixed).not.toContain('\\[^1\\]');
  });
});

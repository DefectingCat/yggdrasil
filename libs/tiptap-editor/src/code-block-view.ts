import type { Editor } from '@tiptap/core';
import type { Node as PMNode } from '@tiptap/pm/model';
import type { ViewMutationRecord } from '@tiptap/pm/view';
import { extractLang, extractOverridesJson } from './highlight';
import { openRunnableModal } from './slash-command';

/** editor.storage 的 key，宿主（index.ts）在此注入 onRunCode 回调。 */
export const ON_RUN_CODE_STORAGE_KEY = '__onRunCode';

/** 运行请求参数（传给 onRunCode 回调）。 */
export interface RunCodeOpts {
  /** 纯语言名（extractLang 提取，如 "python"）。 */
  language: string;
  /** 代码块文本内容。 */
  source: string;
  /** overrides 的 JSON 字符串（extractOverridesJson 提取，空串表示无 overrides）。 */
  overridesJson: string;
}

/**
 * 判断节点是否为 runnable 代码块（language 属性含 'runnable' 标记）。
 */
function isRunnable(node: PMNode): boolean {
  const lang = (node.attrs.language as string | null) ?? '';
  return lang.includes('runnable');
}

/**
 * CodeBlock 自定义 NodeView。
 *
 * 在标准 `<pre><code>` 外包一层容器，顶部加工具栏（语言标签 + 运行按钮），
 * 底部加运行结果区。contentDOM 仍指向 `<code>`，保证 CodeBlockLowlight 的
 * decoration（语法高亮）正常生效。
 *
 * 运行按钮仅 runnable 块显示；点击调 editor.storage.__onRunCode 回调
 * （由 index.ts 注入，转发到 Rust server function）。
 */
export class CodeBlockNodeView {
  private node: PMNode;
  private editor: Editor;

  private container: HTMLDivElement;
  private toolbar: HTMLDivElement;
  private langBadge: HTMLSpanElement;
  private runBtn: HTMLButtonElement | null = null;
  private pre: HTMLPreElement;
  private code: HTMLElement;
  private resultArea: HTMLDivElement | null = null;

  private getPos: (() => number | undefined) | undefined;

  constructor(opts: { node: PMNode; editor: Editor; getPos?: () => number | undefined }) {
    this.node = opts.node;
    this.editor = opts.editor;
    this.getPos = opts.getPos;

    this.container = document.createElement('div');
    this.container.classList.add('tiptap-codeblock');

    // 工具栏
    this.toolbar = document.createElement('div');
    this.toolbar.classList.add('tiptap-codeblock-toolbar');
    this.toolbar.setAttribute('contenteditable', 'false');

    this.langBadge = document.createElement('span');
    this.langBadge.classList.add('tiptap-codeblock-lang');
    this.langBadge.textContent = extractLang((this.node.attrs.language as string) ?? '');
    // runnable 块的语言标签可点击，触发编辑模态框（改语言/overrides）
    if (isRunnable(this.node)) {
      this.langBadge.classList.add('tiptap-codeblock-lang-editable');
      this.langBadge.title = '点击修改语言与运行配置';
      this.langBadge.addEventListener('click', () => this.openEditModal());
    }
    this.toolbar.appendChild(this.langBadge);

    // 运行按钮（仅 runnable 块）
    if (isRunnable(this.node)) {
      this.runBtn = this.createRunButton();
      this.toolbar.appendChild(this.runBtn);
    }

    this.container.appendChild(this.toolbar);

    // pre > code（contentDOM，decoration 在此生效）
    this.pre = document.createElement('pre');
    this.code = document.createElement('code');
    // 只挂纯语言名的 class（extractLang 提取首个 token）。
    // 不能用完整 info string——`python runnable {...}` 含空格，classList.add 会抛
    // InvalidCharacterError。高亮靠 CodeBlockLowlight 的 decoration，不依赖此 class。
    const langClass = extractLang((this.node.attrs.language as string) ?? '');
    if (langClass) {
      this.code.classList.add(`language-${langClass}`);
    }
    this.pre.appendChild(this.code);
    this.container.appendChild(this.pre);
  }

  get dom(): HTMLElement {
    return this.container;
  }

  get contentDOM(): HTMLElement | null {
    // contentDOM 必须是 <code>，ProseMirror 在此挂 decoration + 处理文本编辑。
    return this.code;
  }

  /** ProseMirror 调用：节点属性变化时刷新工具栏。返回 false 拒绝非同类节点。 */
  update(node: PMNode): boolean {
    // 按 node.type 引用比较：真实 ProseMirror 每个 schema 的 NodeType 是单例，
    // 同类节点引用必等（对齐 upload-image.ts 既有范式）。
    if (node.type !== this.node.type) return false;
    const oldLang = (this.node.attrs.language as string) ?? '';
    const newLang = (node.attrs.language as string) ?? '';
    this.node = node;
    if (oldLang !== newLang) {
      this.langBadge.textContent = extractLang(newLang);
      // 更新 <code> 的 language class（低亮按新语言重算）
      this.code.className = '';
      const langClass = extractLang(newLang);
      if (langClass) this.code.classList.add(`language-${langClass}`);
      // runnable 状态变化时重建按钮（简化：不细粒度增删，整体重建工具栏按钮区）
      this.refreshRunButton();
    }
    return true;
  }

  /** 语言变化后，按 runnable 状态增删运行按钮。 */
  private refreshRunButton(): void {
    const shouldHave = isRunnable(this.node);
    const has = this.runBtn !== null;
    if (shouldHave && !has) {
      this.runBtn = this.createRunButton();
      this.toolbar.appendChild(this.runBtn);
    } else if (!shouldHave && has) {
      this.runBtn?.remove();
      this.runBtn = null;
    }
  }

  /**
   * 判断 DOM mutation 是否应被 ProseMirror 忽略。
   *
   * contentDOM（<code>）内的编辑（characterData/childList）必须返回 false，
   * 让 ProseMirror 正常处理事务——否则输入/退格会让文档状态与 DOM 失同步
   * （表现为 Backspace 删整块而非删字符）。
   *
   * 仅对工具栏/结果区等装饰元素返回 true（避免它们的变化触发无谓的事务）。
   * 对齐 Tiptap NodeViewWrapper 默认 ignoreMutation 逻辑。
   */
  ignoreMutation(mutation: ViewMutationRecord): boolean {
    // selection：让 ProseMirror 管光标
    if (mutation.type === 'selection') return false;
    // contentDOM 自身的 attributes 变化（如高亮 decoration 改 class）：忽略
    if (mutation.target === this.contentDOM && mutation.type === 'attributes') return true;
    // contentDOM 内的 mutation（characterData/childList）：交给 ProseMirror（编辑核心）
    // 注意 contains 对 target===contentDOM 自身也返回 true，故上面的 attributes 判断须在前。
    if (this.contentDOM && this.contentDOM.contains(mutation.target)) return false;
    // 工具栏/结果区等装饰元素的 mutation：忽略
    return true;
  }

  /** 工具栏按钮点击不被编辑器拦截。 */
  stopEvent(): boolean {
    return false;
  }

  /** 创建运行按钮（构造与 update 增删共用）。 */
  private createRunButton(): HTMLButtonElement {
    const btn = document.createElement('button');
    btn.classList.add('tiptap-codeblock-run');
    btn.type = 'button';
    btn.textContent = '▶ 运行';
    btn.setAttribute('contenteditable', 'false');
    btn.addEventListener('click', () => this.runCode());
    return btn;
  }

  /** 点击运行：调 editor.storage.__onRunCode，结果填入结果区。 */
  /** 点击语言标签：打开编辑模态框，修改当前 runnable 块的语言/overrides。 */
  private openEditModal(): void {
    const pos = this.getPos?.();
    const currentInfo = (this.node.attrs.language as string) ?? '';
    if (pos === undefined) return;
    openRunnableModal(this.editor, pos, currentInfo);
  }

  private async runCode(): Promise<void> {
    if (!this.runBtn) return;
    const storage = this.editor.storage as unknown as Record<string, unknown>;
    const onRunCode = storage[ON_RUN_CODE_STORAGE_KEY] as
      | ((opts: RunCodeOpts) => Promise<string>)
      | undefined;
    if (!onRunCode) return; // 优雅降级：无回调时不操作

    // 防重复点击
    this.runBtn.disabled = true;
    this.runBtn.textContent = '运行中…';
    this.ensureResultArea('运行中…');

    try {
      const info = (this.node.attrs.language as string) ?? '';
      const result = await onRunCode({
        language: extractLang(info),
        source: this.node.textContent,
        overridesJson: extractOverridesJson(info),
      });
      this.renderResult(result);
    } catch (e) {
      this.renderResult(`运行失败：${e instanceof Error ? e.message : String(e)}`);
    } finally {
      this.runBtn.disabled = false;
      this.runBtn.textContent = '▶ 运行';
    }
  }

  /** 确保 resultArea 存在，设置初始内容。 */
  private ensureResultArea(initial: string): void {
    if (!this.resultArea) {
      this.resultArea = document.createElement('div');
      this.resultArea.classList.add('tiptap-codeblock-result');
      this.resultArea.setAttribute('contenteditable', 'false');
      this.container.appendChild(this.resultArea);
    }
    this.resultArea.textContent = initial;
  }

  /** 渲染运行结果字符串到结果区。 */
  private renderResult(result: string): void {
    this.ensureResultArea(result);
  }

  destroy(): void {
    this.resultArea = null;
    this.runBtn = null;
  }
}

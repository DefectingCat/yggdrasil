import { type Editor, Extension, type Range } from '@tiptap/core';
import { PluginKey } from '@tiptap/pm/state';
import { Suggestion, type SuggestionKeyDownProps, type SuggestionProps } from '@tiptap/suggestion';

interface CommandItem {
  title: string;
  description: string;
  icon: string;
  command: (props: { editor: Editor; range: Range }) => void;
  /**
   * 搜索别名（空格分隔），让中英文都能命中。
   * 例：「代码块」keywords='code codeblock' → /code 与 /代码 都匹配。
   * title/description 已含的字词不必重复写（过滤逻辑会一并匹配）。
   */
  keywords?: string;
}

/**
 * 判断命令是否匹配搜索词（不区分大小写）。
 *
 * 命中 title / description / keywords 任一即算匹配。keywords 是空格分隔的别名
 * （含英文/常见词），让中英文互通：`/code` 能命中「代码块」，`/代码` 也能。
 * 抽成纯函数便于单元测试。
 */
export function matchCommand(item: CommandItem, query: string): boolean {
  const q = query.toLowerCase();
  return (
    item.title.toLowerCase().includes(q) ||
    item.description.toLowerCase().includes(q) ||
    (item.keywords?.toLowerCase().includes(q) ?? false)
  );
}

/**
 * 斜杠命令扩展的选项。
 *
 * `onImageUpload` 由宿主注入（参见 index.ts），用于把用户选择的图片文件
 * 上传到服务端并返回可访问的 URL。未提供时"上传图片"命令会被隐藏，
 * 只保留"图片链接"（手动填 URL）。
 */
export interface SlashCommandOptions {
  onImageUpload?: (file: File) => Promise<string>;
  /** 由 index.ts 注入：直接调 coordinator.insertUploading（走占位符 + 上传）。 */
  onInsertUploading?: (file: File) => void;
}

const SlashCommandPluginKey = new PluginKey('slashCommand');

/**
 * 斜杠命令扩展。
 *
 * `onImageUpload` 通过 `addOptions` 注入，"上传图片"命令据此决定是否出现。
 */
export const SlashCommand = Extension.create<SlashCommandOptions>({
  name: 'slashCommand',

  addOptions() {
    return {
      onImageUpload: undefined,
      onInsertUploading: undefined,
    };
  },

  addProseMirrorPlugins() {
    // 依据是否提供上传回调，决定可用命令集。
    const uploadFn = this.options.onImageUpload;
    const COMMANDS: CommandItem[] = [
      {
        title: '标题 1',
        description: '大标题',
        icon: 'H1',
        keywords: 'h1 heading 标题',
        command: ({ editor, range }) => {
          editor.chain().focus().deleteRange(range).setHeading({ level: 1 }).run();
        },
      },
      {
        title: '标题 2',
        description: '中标题',
        icon: 'H2',
        keywords: 'h2 heading 标题',
        command: ({ editor, range }) => {
          editor.chain().focus().deleteRange(range).setHeading({ level: 2 }).run();
        },
      },
      {
        title: '标题 3',
        description: '小标题',
        icon: 'H3',
        keywords: 'h3 heading 标题',
        command: ({ editor, range }) => {
          editor.chain().focus().deleteRange(range).setHeading({ level: 3 }).run();
        },
      },
      {
        title: '无序列表',
        description: '创建无序列表',
        icon: '•',
        keywords: 'bullet list ul 列表',
        command: ({ editor, range }) => {
          editor.chain().focus().deleteRange(range).toggleBulletList().run();
        },
      },
      {
        title: '有序列表',
        description: '创建有序列表',
        icon: '1.',
        keywords: 'ordered ol number list 列表',
        command: ({ editor, range }) => {
          editor.chain().focus().deleteRange(range).toggleOrderedList().run();
        },
      },
      {
        title: '任务列表',
        description: '创建任务列表',
        icon: '☑',
        keywords: 'task todo checklist 列表',
        command: ({ editor, range }) => {
          editor.chain().focus().deleteRange(range).toggleTaskList().run();
        },
      },
      {
        title: '引用',
        description: '插入引用块',
        icon: '❝',
        keywords: 'quote blockquote 引用',
        command: ({ editor, range }) => {
          editor.chain().focus().deleteRange(range).toggleBlockquote().run();
        },
      },
      {
        title: '代码块',
        description: '插入代码块',
        icon: '<>',
        keywords: 'code codeblock pre 代码',
        command: ({ editor, range }) => {
          editor.chain().focus().deleteRange(range).toggleCodeBlock().run();
        },
      },
      {
        title: '可运行代码块',
        description: '插入可被读者执行的代码块',
        icon: '▶',
        keywords: 'code run runnable execute 代码 运行',
        command: ({ editor, range }) => {
          editor.chain().focus().deleteRange(range).run();
          openRunnableModal(editor);
        },
      },
      {
        title: '分割线',
        description: '插入水平分割线',
        icon: '—',
        keywords: 'hr rule divider 分割',
        command: ({ editor, range }) => {
          editor.chain().focus().deleteRange(range).setHorizontalRule().run();
        },
      },
      {
        title: '表格',
        description: '插入 3×3 表格',
        icon: '▦',
        keywords: 'table 表格',
        command: ({ editor, range }) => {
          editor
            .chain()
            .focus()
            .deleteRange(range)
            .insertTable({ rows: 3, cols: 3, withHeaderRow: true })
            .run();
        },
      },
    ];

    // 图片相关命令：上传命令仅在上传回调可用时才出现。
    if (uploadFn) {
      COMMANDS.push({
        title: '上传图片',
        description: '从本地选择并上传图片',
        icon: '📤',
        keywords: 'image upload 图片',
        command: ({ editor, range }) => {
          // 必须先删掉 /命令 文本，文件选择对话框会阻塞，关闭后 range 可能失效。
          editor.chain().focus().deleteRange(range).run();
          const input = document.createElement('input');
          input.type = 'file';
          input.accept = 'image/jpeg,image/png,image/gif,image/webp';
          input.addEventListener('change', () => {
            const file = input.files?.[0];
            if (!file) return;
            // 优先走 coordinator（占位符 + 上传），否则退回直接上传（无占位符）
            if (this.options.onInsertUploading) {
              this.options.onInsertUploading(file);
            } else if (uploadFn) {
              uploadFn(file)
                .then((url) => {
                  editor.chain().focus().setImage({ src: url }).run();
                })
                .catch((err) => {
                  const msg = err instanceof Error ? err.message : String(err);
                  console.error('[SlashCommand] Upload failed:', msg);
                });
            }
          });
          // click() 会立即触发原生文件选择器；回调在用户选择文件后异步执行。
          input.click();
        },
      });
    }

    COMMANDS.push(
      {
        title: '图片链接',
        description: '通过 URL 插入图片',
        icon: '🖼',
        keywords: 'image url 图片',
        command: ({ editor, range }) => {
          const url = window.prompt('输入图片 URL');
          if (url && isValidUrl(url)) {
            editor.chain().focus().deleteRange(range).setImage({ src: url }).run();
          }
        },
      },
      {
        title: '链接',
        description: '插入链接',
        icon: '🔗',
        keywords: 'link url a href 链接',
        command: ({ editor, range }) => {
          const url = window.prompt('输入链接 URL');
          if (!url || !isValidUrl(url)) return;
          // deleteRange 后光标停在 range.to；先插入 URL 文本，再选中刚插入的范围设 link
          // （setLink 需要非空选区才生效，原顺序 setLink 在空选区无效）。
          const insertFrom = range.to;
          editor
            .chain()
            .focus()
            .deleteRange(range)
            .insertContent(url)
            .setTextSelection({ from: insertFrom, to: insertFrom + url.length })
            .setLink({ href: url })
            .run();
        },
      },
    );

    return [
      Suggestion<CommandItem>({
        pluginKey: SlashCommandPluginKey,
        editor: this.editor,
        char: '/',
        items: ({ query }) => {
          return COMMANDS.filter((item) => matchCommand(item, query));
        },
        render() {
          let popup: SlashPopup | null = null;

          return {
            onStart(props) {
              popup = createPopup(props);
            },
            onUpdate(props) {
              if (!popup) return;
              popup.updateItems(props.items);
              popup.updatePosition();
            },
            onKeyDown(props) {
              if (!popup) return false;
              return popup.onKeyDown(props);
            },
            onExit() {
              if (popup) {
                popup.destroy();
                popup = null;
              }
            },
          };
        },
        command: ({ editor, range, props: item }) => {
          item.command({ editor, range });
        },
      }),
    ];
  },
});

/** 斜杠命令浮层实例:供 Suggestion render 生命周期驱动。 */
interface SlashPopup {
  component: HTMLElement;
  updateItems(items: CommandItem[]): void;
  updatePosition(): void;
  onKeyDown(props: SuggestionKeyDownProps): boolean;
  destroy(): void;
}

/** 校验图片/链接 URL:只允许 http(s) 和 data:image。拒绝 javascript: 等。 */
export function isValidUrl(url: string): boolean {
  return /^https?:\/\//i.test(url) || /^data:image\//i.test(url);
}

export function createPopup(props: SuggestionProps<CommandItem>): SlashPopup {
  const component = document.createElement('div');
  component.classList.add('slash-command');

  const list = document.createElement('div');
  list.classList.add('slash-command-list');
  component.appendChild(list);

  let selectedIndex = 0;
  let currentItems: CommandItem[] = [];

  function renderItems(items: CommandItem[]) {
    currentItems = items;
    list.innerHTML = '';
    selectedIndex = 0;

    // 空状态：显示提示，不渲染列表项。
    if (items.length === 0) {
      const empty = document.createElement('div');
      empty.classList.add('slash-command-empty');
      empty.textContent = '无匹配命令';
      list.appendChild(empty);
      return;
    }

    items.forEach((item, index) => {
      const el = document.createElement('div');
      el.classList.add('slash-command-item');
      if (index === 0) el.classList.add('is-selected');

      el.innerHTML = `
        <div class="slash-command-item-icon">${item.icon}</div>
        <div class="slash-command-item-text">
          <div class="slash-command-item-title">${item.title}</div>
          <div class="slash-command-item-desc">${item.description}</div>
        </div>
      `;

      el.addEventListener('click', () => {
        props.command(item);
      });

      el.addEventListener('mouseenter', () => {
        selectedIndex = index;
        updateSelection();
      });

      list.appendChild(el);
    });
  }

  function updateSelection() {
    const children = list.children;
    for (let i = 0; i < children.length; i++) {
      if (i === selectedIndex) {
        children[i].classList.add('is-selected');
      } else {
        children[i].classList.remove('is-selected');
      }
    }
    children[selectedIndex]?.scrollIntoView({ block: 'nearest' });
  }

  function selectItem() {
    if (currentItems[selectedIndex]) {
      props.command(currentItems[selectedIndex]);
    }
  }

  function updatePosition() {
    const rect = props.clientRect?.();
    if (!rect) return;
    component.style.left = `${rect.left}px`;
    component.style.top = `${rect.bottom + 4}px`;
  }

  renderItems(props.items);
  document.body.appendChild(component);
  updatePosition();

  return {
    component,
    updateItems(items: CommandItem[]) {
      renderItems(items);
    },
    updatePosition,
    onKeyDown({ event }: SuggestionKeyDownProps): boolean {
      // 空列表时不拦截键盘：避免 % 0 产生 NaN，也避免吞掉 Enter（让用户正常输入）。
      // Escape 仍拦截（关闭浮层）。
      if (event.key === 'Escape') {
        event.preventDefault();
        return true;
      }
      if (currentItems.length === 0) {
        return false;
      }
      if (event.key === 'ArrowUp') {
        event.preventDefault();
        selectedIndex = (selectedIndex - 1 + currentItems.length) % currentItems.length;
        updateSelection();
        return true;
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        selectedIndex = (selectedIndex + 1) % currentItems.length;
        updateSelection();
        return true;
      }
      if (event.key === 'Enter') {
        event.preventDefault();
        selectItem();
        return true;
      }
      return false;
    },
    destroy() {
      component.remove();
    },
  };
}

/** buildRunnableInfo 的输入配置。 */
export interface RunnableInfoOpts {
  /** 语言名（python / node）。 */
  lang: string;
  /** 超时秒数。 */
  timeoutSecs: number;
  /** 内存上限（MB）。 */
  memoryMb: number;
  /** 是否允许网络。 */
  allowNetwork: boolean;
  /** 作者是否改动过任一 overrides 字段；false 则省略 JSON。 */
  dirty: boolean;
}

/**
 * 把弹框收集的配置转成 markdown fence 的 info string。
 *
 * - dirty=false → `${lang} runnable`（省略 JSON，最小形态）
 * - dirty=true  → `${lang} runnable {"timeout_secs":N,"memory_mb":M,"allow_network":B}`
 *
 * JSON 字段顺序固定（timeout → memory → network），由显式构造保证（不依赖对象插入顺序）。
 * 到达此函数时值必然合法（弹框「插入」按钮在非法值时 disabled）。
 */
export function buildRunnableInfo(opts: RunnableInfoOpts): string {
  const prefix = `${opts.lang} runnable`;
  if (!opts.dirty) return prefix;
  // 显式拼字符串，保证字段顺序固定（timeout → memory → network），不依赖对象键序。
  const json = `{"timeout_secs":${opts.timeoutSecs},"memory_mb":${opts.memoryMb},"allow_network":${opts.allowNetwork}}`;
  return `${prefix} ${json}`;
}

/**
 * 受支持的语言（与 src/pages/admin/runner.rs SUPPORTED_LANGS 对齐）。
 * 编辑器是纯 JS lib，不调 server function，故写死。
 */
const RUNNABLE_LANGS = ['python', 'node'] as const;

/** 模态框默认值（与后端 ResourceLimits 默认对齐：见 languages.rs）。 */
const RUNNABLE_DEFAULTS = { timeoutSecs: 5, memoryMb: 256, allowNetwork: false };

/** timeout_secs 取值范围（与 CODE_RUNNER_MAX_TIMEOUT_SECS 对齐）。 */
const TIMEOUT_RANGE = { min: 1, max: 30 } as const;
/** memory_mb 取值范围（与 CODE_RUNNER_MAX_MEMORY_MB 对齐）。 */
const MEMORY_RANGE = { min: 16, max: 1024 } as const;

/**
 * 打开「插入可运行代码块」模态框。
 *
 * 作者选语言 + 可选 overrides（超时/内存/网络），确认后调用
 * `editor.chain().focus().setCodeBlock({ language }).run()` 插入标准 CodeBlock，
 * 其 language 属性承载完整 'python runnable {...}' info string（marked 往返保真）。
 *
 * 任一 overrides 字段被改动即 dirty；dirty=false 插入 'python runnable'（无 JSON）。
 * Esc / 遮罩点击 / 取消按钮 → 关闭不插入。
 */
export function openRunnableModal(editor: Editor): void {
  const state = { ...RUNNABLE_DEFAULTS, lang: 'python' as string, dirty: false };

  const mask = document.createElement('div');
  mask.className = 'tiptap-runnable-modal-mask';

  const modal = document.createElement('div');
  modal.className = 'tiptap-runnable-modal';

  const title = document.createElement('div');
  title.className = 'tiptap-runnable-modal-title';
  title.textContent = '插入可运行代码块';
  modal.appendChild(title);

  // 语言选择
  const langRow = document.createElement('label');
  langRow.className = 'tiptap-runnable-field';
  langRow.textContent = '语言';
  const langSelect = document.createElement('select');
  langSelect.id = 'runnable-lang';
  for (const l of RUNNABLE_LANGS) {
    const opt = document.createElement('option');
    opt.value = l;
    opt.textContent = l;
    langSelect.appendChild(opt);
  }
  langSelect.value = state.lang;
  langSelect.addEventListener('change', () => {
    state.lang = langSelect.value;
    updatePreview();
  });
  langRow.appendChild(langSelect);
  modal.appendChild(langRow);

  // 超时
  const timeoutRow = document.createElement('label');
  timeoutRow.className = 'tiptap-runnable-field';
  timeoutRow.textContent = '超时（秒）';
  const timeoutInput = document.createElement('input');
  timeoutInput.id = 'runnable-timeout';
  timeoutInput.type = 'number';
  timeoutInput.min = String(TIMEOUT_RANGE.min);
  timeoutInput.max = String(TIMEOUT_RANGE.max);
  timeoutInput.value = String(state.timeoutSecs);
  timeoutInput.addEventListener('input', () => {
    state.timeoutSecs = Number(timeoutInput.value) || RUNNABLE_DEFAULTS.timeoutSecs;
    state.dirty = true;
    updatePreview();
    updateInsertEnabled();
  });
  timeoutRow.appendChild(timeoutInput);
  modal.appendChild(timeoutRow);

  // 内存
  const memRow = document.createElement('label');
  memRow.className = 'tiptap-runnable-field';
  memRow.textContent = '内存（MB）';
  const memInput = document.createElement('input');
  memInput.id = 'runnable-memory';
  memInput.type = 'number';
  memInput.min = String(MEMORY_RANGE.min);
  memInput.max = String(MEMORY_RANGE.max);
  memInput.value = String(state.memoryMb);
  memInput.addEventListener('input', () => {
    state.memoryMb = Number(memInput.value) || RUNNABLE_DEFAULTS.memoryMb;
    state.dirty = true;
    updatePreview();
    updateInsertEnabled();
  });
  memRow.appendChild(memInput);
  modal.appendChild(memRow);

  // 网络
  const netRow = document.createElement('label');
  netRow.className = 'tiptap-runnable-field';
  const netInput = document.createElement('input');
  netInput.id = 'runnable-network';
  netInput.type = 'checkbox';
  netInput.checked = state.allowNetwork;
  netInput.addEventListener('change', () => {
    state.allowNetwork = netInput.checked;
    state.dirty = true;
    updatePreview();
  });
  netRow.appendChild(netInput);
  netRow.appendChild(document.createTextNode('允许网络'));
  modal.appendChild(netRow);

  // 预览
  const preview = document.createElement('div');
  preview.className = 'tiptap-runnable-preview';
  modal.appendChild(preview);

  // 按钮
  const actions = document.createElement('div');
  actions.className = 'tiptap-runnable-actions';
  const cancelBtn = document.createElement('button');
  cancelBtn.className = 'cancel';
  cancelBtn.type = 'button';
  cancelBtn.textContent = '取消';
  cancelBtn.addEventListener('click', close);
  const insertBtn = document.createElement('button');
  insertBtn.className = 'insert';
  insertBtn.type = 'button';
  insertBtn.textContent = '插入';
  insertBtn.addEventListener('click', insert);
  actions.appendChild(cancelBtn);
  actions.appendChild(insertBtn);
  modal.appendChild(actions);

  mask.appendChild(modal);
  mask.addEventListener('click', (e) => {
    // 仅点击遮罩本身（非卡片）时关闭
    if (e.target === mask) close();
  });

  function updatePreview(): void {
    preview.textContent = `\`\`\`${buildRunnableInfo(state)}`;
  }

  /** 校验数字字段：全合法才启用「插入」。 */
  function updateInsertEnabled(): void {
    const t = Number(timeoutInput.value);
    const m = Number(memInput.value);
    insertBtn.disabled = !(
      t >= TIMEOUT_RANGE.min &&
      t <= TIMEOUT_RANGE.max &&
      m >= MEMORY_RANGE.min &&
      m <= MEMORY_RANGE.max
    );
  }

  function insert(): void {
    editor
      .chain()
      .setCodeBlock({ language: buildRunnableInfo(state) })
      .run();
    close();
  }

  function close(): void {
    document.removeEventListener('keydown', onKeydown);
    mask.remove();
    editor.chain().focus().run();
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'Enter' && !insertBtn.disabled) {
      // Enter 在表单元素内提交（浏览器原生 number input 的 Enter 不会触发 click）。
      // 注意：网络 checkbox 的 tagName 也是 'input'，Enter 会触发提交而非切换
      // （checkbox 原生用 Space 切换），符合模态框 Enter=确认的惯例。
      // 例外：语言 <select> 下拉展开时 Enter 用于确认选项，不应触发插入。
      // （HTMLSelectElement.open 在浏览器存在，但 TS lib.dom 未声明，需断言。）
      if ((langSelect as HTMLSelectElement & { open: boolean }).open) return;
      const tag = (document.activeElement?.tagName ?? '').toLowerCase();
      if (tag === 'input' || tag === 'select') {
        e.preventDefault();
        insert();
      }
    }
  }

  document.addEventListener('keydown', onKeydown);
  document.body.appendChild(mask);
  updatePreview();
  updateInsertEnabled();
  langSelect.focus();
}

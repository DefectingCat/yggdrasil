import { javascript } from '@codemirror/lang-javascript';
import { python } from '@codemirror/lang-python';
import { PostgreSQL, sql } from '@codemirror/lang-sql';
import { Compartment, EditorState, type Extension, Prec } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { vim } from '@replit/codemirror-vim';
import { basicSetup } from 'codemirror';
import { type ThemeName, themeExtension } from './themes';

/** SQL 补全用 schema 数据（由 Rust 侧从实时库拉取注入）。 */
export interface SqlSchema {
  tables: { name: string; columns: string[] }[];
}

/**
 * 传给 CodeMirrorEditor.create 的配置。
 * 必须是 class（非 interface），以便 TS 擦除后存活，
 * wasm 侧能用 `new EditorOptions()` 构造，并通过 setter 填充字段。
 */
export class EditorOptions {
  language?: string;
  theme?: ThemeName;
  vim?: boolean;
  schema?: SqlSchema;
  value?: string;
  onChange?: (value: string) => void;
  onReady?: () => void;
  /** Ctrl/Cmd + Enter 快捷键回调（SQL 控制台触发执行）。 */
  onRunShortcut?: () => void;
}

/**
 * 把语言标识映射成 CodeMirror 语言 Extension。
 * - `python` → @codemirror/lang-python
 * - `node` / `javascript` / `js` → @codemirror/lang-javascript
 * - `sql` / 缺省 → @codemirror/lang-sql（PostgreSQL 方言 + schema 补全）
 *
 * SQL 分支使用 schema 补全；非 SQL 语言忽略 schema（补全仅对 SQL 有意义）。
 */
function buildLanguageExtension(lang: string, schema: SqlSchema): Extension {
  const normalized = (lang ?? '').toLowerCase();
  if (normalized === 'python') {
    return python();
  }
  if (normalized === 'node' || normalized === 'javascript' || normalized === 'js') {
    return javascript();
  }
  // sql / 缺省：保留原有 PostgreSQL + schema 补全行为
  return sql({
    dialect: PostgreSQL,
    schema: schemaToCompletions(schema),
    upperCaseKeywords: true,
  });
}

/**
 * CodeMirror 实例封装。
 * create() 时用四个 Compartment 注入 theme/language/schema/vim，
 * 支持后续热切换（reconfigure）而不重建实例——保留 Vim 状态、光标、撤销栈。
 */
export class CodeMirrorInstance {
  private view: EditorView;
  private themeCompartment = new Compartment();
  private languageCompartment = new Compartment();
  private vimCompartment = new Compartment();
  private language: string;
  private schema: SqlSchema;

  constructor(container: HTMLElement, options: EditorOptions) {
    const theme: ThemeName = options.theme ?? 'light';
    const schema = options.schema ?? { tables: [] };
    this.schema = schema;
    this.language = options.language ?? 'sql';

    this.view = new EditorView({
      state: EditorState.create({
        doc: options.value ?? '',
        extensions: [
          basicSetup,
          // Ctrl/Cmd + Enter 运行快捷键：用 Prec.highest 包裹，保证在 vim 的
          // ViewPlugin 按键拦截之前命中（vim 自己也用 Prec.highest，跟随此惯例）。
          ...(options.onRunShortcut
            ? [
                Prec.highest(
                  keymap.of([
                    {
                      key: 'Mod-Enter',
                      preventDefault: true,
                      run: () => {
                        options.onRunShortcut?.();
                        return true;
                      },
                    },
                  ]),
                ),
              ]
            : []),
          // vim 必须在 keymap 最前（@replit/codemirror-vim 仓库要求）
          this.vimCompartment.of(options.vim ? [vim()] : []),
          this.themeCompartment.of(themeExtension(theme)),
          this.languageCompartment.of(buildLanguageExtension(this.language, schema)),
          EditorView.updateListener.of((v) => {
            if (v.docChanged) {
              options.onChange?.(this.view.state.doc.toString());
            }
          }),
        ],
      }),
      parent: container,
    });

    options.onReady?.();
  }

  getValue(): string {
    return this.view.state.doc.toString();
  }

  setValue(s: string): void {
    this.view.dispatch({
      changes: { from: 0, to: this.view.state.doc.length, insert: s },
    });
  }

  /** 热切换主题，不重建实例（Compartment.reconfigure）。 */
  setTheme(theme: ThemeName): void {
    this.view.dispatch({
      effects: this.themeCompartment.reconfigure(themeExtension(theme)),
    });
  }

  /** 热切换语言（python/node/javascript/sql），不重建实例（Compartment.reconfigure）。 */
  setLanguage(lang: string): void {
    this.language = lang;
    this.view.dispatch({
      effects: this.languageCompartment.reconfigure(buildLanguageExtension(lang, this.schema)),
    });
  }

  /** 更新 SQL 补全 schema（Compartment.reconfigure）。
   *  当前语言为 SQL 时立即生效；非 SQL 语言会缓存 schema，切回 SQL 时生效。 */
  setSchema(schema: SqlSchema): void {
    this.schema = schema;
    // 仅当当前是 SQL 语言时才重配 extension（其它语言不消费 schema）。
    if ((this.language ?? '').toLowerCase() === 'sql' || !this.language) {
      this.view.dispatch({
        effects: this.languageCompartment.reconfigure(buildLanguageExtension('sql', schema)),
      });
    }
  }

  focus(): void {
    this.view.focus();
  }

  destroy(): void {
    this.view.destroy();
  }
}

/** 把 SqlSchema 转成 @codemirror/lang-sql 期望的补全结构。 */
function schemaToCompletions(schema: SqlSchema) {
  return schema.tables.map((t) => ({
    label: t.name,
    type: 'table',
    detail: 'table',
    columns: t.columns.map((c) => ({ label: c, type: 'column' })),
  }));
}

// 暴露 EditorOptions 到 window，供 wasm-bindgen 用 new EditorOptions()。
// IIFE 的 name 只能挂一个全局（CodeMirrorEditor），故手动 hoist EditorOptions。
declare global {
  interface Window {
    EditorOptions: typeof EditorOptions;
  }
}
window.EditorOptions = EditorOptions;

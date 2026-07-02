import { PostgreSQL, sql } from '@codemirror/lang-sql';
import { Compartment, EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
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
}

/**
 * CodeMirror 实例封装。
 * create() 时用三个 Compartment 注入 theme/schema/vim，
 * 支持后续热切换（reconfigure）而不重建实例——保留 Vim 状态、光标、撤销栈。
 */
export class CodeMirrorInstance {
  private view: EditorView;
  private themeCompartment = new Compartment();
  private schemaCompartment = new Compartment();
  private vimCompartment = new Compartment();

  constructor(container: HTMLElement, options: EditorOptions) {
    const theme: ThemeName = options.theme ?? 'light';
    const schema = options.schema ?? { tables: [] };

    this.view = new EditorView({
      state: EditorState.create({
        doc: options.value ?? '',
        extensions: [
          basicSetup,
          // vim 必须在 keymap 最前（@replit/codemirror-vim 仓库要求）
          this.vimCompartment.of(options.vim ? [vim()] : []),
          this.themeCompartment.of(themeExtension(theme)),
          this.schemaCompartment.of(
            sql({
              dialect: PostgreSQL,
              schema: schemaToCompletions(schema),
              upperCaseKeywords: true,
            }),
          ),
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

  /** 更新 SQL 补全 schema（Compartment.reconfigure）。 */
  setSchema(schema: SqlSchema): void {
    this.view.dispatch({
      effects: this.schemaCompartment.reconfigure(
        sql({
          dialect: PostgreSQL,
          schema: schemaToCompletions(schema),
          upperCaseKeywords: true,
        }),
      ),
    });
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

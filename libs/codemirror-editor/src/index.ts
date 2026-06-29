import { CodeMirrorInstance, EditorOptions } from './editor';

/**
 * 模块入口：暴露对象字面量 { create } 作为默认导出。
 * IIFE 产物挂在 window.CodeMirrorEditor 上，由 Rust 侧用 Reflect::get 取
 * （对象字面量，不能用 wasm-bindgen 的 extern fn——那会被编成函数调用而失败）。
 */
const CodeMirrorEditor = {
  _instances: new Map<string, CodeMirrorInstance>(),

  create(
    containerId: string,
    options: EditorOptions = new EditorOptions(),
  ): CodeMirrorInstance | null {
    const container = document.getElementById(containerId);
    if (!container) return null;

    // 销毁同 id 的旧实例
    const existing = this._instances.get(containerId);
    if (existing) {
      existing.destroy();
      this._instances.delete(containerId);
    }

    const instance = new CodeMirrorInstance(container, options);
    this._instances.set(containerId, instance);
    return instance;
  },
};

export default CodeMirrorEditor;

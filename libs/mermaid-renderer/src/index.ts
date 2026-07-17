// 把 mermaid 包成 IIFE 全局 bundle。
//
// 关键：vite IIFE + `export default X` 会产出 `window.MermaidRenderer.default = X`
// （多一层 .default）。为避免 mermaid.ts 取值时纠结这一层，这里把 mermaid 对象
// 直接赋给 window.MermaidRenderer 作为构建时副作用，使 window.MermaidRenderer
// 本身即 mermaid API（initialize/render）。
import mermaid from 'mermaid';
// 构建时副作用：把 mermaid 挂到全局。ES module 的顶层赋值会被 vite 保留。
(globalThis as unknown as { MermaidRenderer: typeof mermaid }).MermaidRenderer = mermaid;

export default mermaid;

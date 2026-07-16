// 把 mermaid 默认导出包成 IIFE 全局 bundle。
// yggdrasil-core 的 mermaid.ts 通过动态 import('/mermaid/mermaid.js') 拿到 .default。
import mermaid from 'mermaid';

export default mermaid;

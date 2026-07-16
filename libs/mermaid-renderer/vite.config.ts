import { resolve } from 'node:path';
import { defineConfig } from 'vite';

// mermaid 独立 IIFE bundle：~1MB，不打进 yggdrasil-core（避免拖累每篇文章首屏）。
// 由 yggdrasil-core 的 mermaid.ts 在 IntersectionObserver 视口可见时动态 import。
// 输出到 public/mermaid/mermaid.js，dx build 会作为静态资源拷贝。
export default defineConfig({
  build: {
    outDir: resolve(__dirname, '../../public/mermaid'),
    emptyOutDir: true,
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
      name: 'MermaidRenderer',
      fileName: () => 'mermaid.js',
      formats: ['iife'],
    },
    // mermaid 全量打进单文件（含其依赖 cytoscape/dagre-d3 等）。
    rolldownOptions: {
      output: {
        inlineDynamicImports: true,
      },
    },
    cssCodeSplit: false,
    minify: true,
    sourcemap: true,
  },
});

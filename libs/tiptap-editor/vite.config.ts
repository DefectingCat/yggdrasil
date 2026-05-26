import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  build: {
    outDir: resolve(__dirname, '../../public/tiptap'),
    emptyOutDir: true,
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
      name: 'TiptapEditor',
      fileName: 'editor',
      formats: ['iife'],
    },
    rollupOptions: {
      output: {
        exports: 'default',
        assetFileNames: 'editor.[ext]',
        inlineDynamicImports: true,
      },
    },
    cssCodeSplit: false,
    minify: true,
    sourcemap: true,
  },
});

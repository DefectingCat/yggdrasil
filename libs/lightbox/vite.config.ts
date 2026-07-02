import { resolve } from 'node:path';
import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    outDir: resolve(__dirname, '../../public/lightbox'),
    emptyOutDir: true,
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
      name: 'Lightbox',
      fileName: () => 'lightbox.js',
      formats: ['iife'],
    },
    rolldownOptions: {
      output: {
        assetFileNames: 'lightbox.[ext]',
      },
    },
    cssCodeSplit: false,
    minify: true,
    sourcemap: true,
  },
});

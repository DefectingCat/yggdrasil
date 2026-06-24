import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // 默认 happy-dom：index.ts 的行为测试需要真实 DOM（点击/keydown/scroll/IO）。
    // geometry.test.ts 是纯函数，用文件内 // @vitest-environment node 指令保持 node 环境。
    environment: 'happy-dom',
    include: ['src/**/*.test.ts'],
  },
});

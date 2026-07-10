// 声明 CSS side-effect import，让 tsc --noEmit 不报 TS2882。
// xterm.js 的样式通过 `import '@xterm/xterm/css/xterm.css'` 引入，
// vite cssCodeSplit:false 会把它内联进 IIFE 产物。
declare module '*.css';

/**
 * Mermaid 流程图懒加载渲染。
 *
 * 扫描 `pre > code.language-mermaid` 代码块，在进入视口时动态 import 独立 bundle
 * `public/mermaid/mermaid.js`（~1MB，只在有图且可见时加载，不影响无图文章首屏），
 * 把 mermaid 源码渲染成 SVG 注入到父 <pre>。
 *
 * 范式照搬 post-content.ts：querySelectorAll + 幂等守卫 + 注入 DOM。
 * mermaid 无官方 SSR 支持，纯客户端渲染；服务端 markdown.rs 只产出带
 * `language-mermaid` class 的普通代码块，不挂 data 属性。
 */

import type { ThemeName } from '@yggdrasil/shared';

type MermaidApi = {
  initialize: (config: Record<string, unknown>) => void;
  render: (id: string, text: string) => Promise<{ svg: string }>;
};

let mermaidPromise: Promise<MermaidApi> | null = null;

/**
 * 动态加载 mermaid 独立 bundle 的底层函数（可注入以便测试）。
 *
 * 用绝对路径 '/mermaid/mermaid.js' 动态 import：Vite 无法静态分析此字面量，
 * 故加 @vite-ignore 避免构建时报错，且该路径无类型声明需用函数间接构造 import
 * 字面量以绕过 tsc 的模块解析（TS2307）。bundle 加载后 default export 即 mermaid API。
 * 测试时可通过重赋值 `loadMermaidBundle` 替换为 mock。
 */
export let loadMermaidBundle: () => Promise<MermaidApi> = async () => {
  const url = '/mermaid/mermaid.js';
  const mod = (await import(/* @vite-ignore */ url)) as { default: MermaidApi };
  return mod.default;
};

/** 重置加载函数（测试用，重新注入 mock 后必须重置 mermaidPromise 缓存）。 */
export function _resetMermaidLoader(loader?: () => Promise<MermaidApi>): void {
  mermaidPromise = null;
  if (loader) loadMermaidBundle = loader;
}

/**
 * 动态加载 mermaid 独立 bundle（单例缓存，失败清空允许重试）。
 */
function loadMermaid(): Promise<MermaidApi> {
  if (!mermaidPromise) {
    mermaidPromise = loadMermaidBundle().catch((err) => {
      mermaidPromise = null;
      throw err;
    });
  }
  return mermaidPromise;
}

/**
 * 为单个 mermaid <pre> 注册 IntersectionObserver：进入视口才渲染。
 *
 * 无 IntersectionObserver（SSR / 旧环境）时直接同步渲染。
 * rootMargin 200px 让图在接近视口时提前加载，避免滚到才白屏。
 */
function observeBlock(pre: HTMLPreElement, render: () => Promise<void>): void {
  if (typeof IntersectionObserver === 'undefined') {
    void render();
    return;
  }
  const io = new IntersectionObserver(
    (entries) => {
      if (entries.some((e) => e.isIntersecting)) {
        io.disconnect();
        void render();
      }
    },
    { rootMargin: '200px' },
  );
  io.observe(pre);
}

/**
 * 初始化文章正文里的 mermaid 代码块。
 *
 * @param selector 文章正文容器选择器（如 '.post-content'）
 * @param theme 当前生效主题，传给 mermaid 适配暗色
 */
export function initMermaid(selector: string, theme: ThemeName): void {
  const root = document.querySelector(selector);
  if (!root) return;
  const blocks = root.querySelectorAll<HTMLPreElement>('pre > code.language-mermaid');
  if (blocks.length === 0) return;

  blocks.forEach((code, i) => {
    const pre = code.parentElement as HTMLPreElement | null;
    if (!pre) return;
    if (pre.dataset.mermaidRendered) return; // 幂等：上下篇切换重复调用不重渲染

    const source = code.textContent || '';
    observeBlock(pre, async () => {
      try {
        const mermaid = await loadMermaid();
        mermaid.initialize({
          startOnLoad: false,
          theme: theme === 'dark' ? 'dark' : 'default',
          securityLevel: 'strict',
        });
        const { svg } = await mermaid.render(`mermaid-svg-${i}`, source);
        // 替换整个 <pre> 内容为 SVG，丢弃 copy 按钮（图不需要复制源码）。
        pre.innerHTML = svg;
        pre.dataset.mermaidRendered = 'true';
      } catch (err) {
        // 渲染失败（语法错误 / bundle 加载失败）：保留原始源码，加错误标记 class
        // 便于用户发现是 mermaid 源写错了。不破坏页面其余内容。
        console.error('mermaid render failed:', err);
        pre.classList.add('mermaid-error');
      }
    });
  });
}

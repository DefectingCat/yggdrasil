/**
 * Mermaid 流程图懒加载渲染。
 *
 * 扫描 `pre > code.language-mermaid` 代码块，在进入视口时动态加载独立 IIFE bundle
 * `public/mermaid/mermaid.js`（~1MB，只在有图且可见时加载，不影响无图文章首屏），
 * 把 mermaid 源码渲染成 SVG 注入到父 <pre>。
 *
 * bundle 是 IIFE 格式（与项目其他前端库一致），挂全局变量 `window.MermaidRenderer`。
 * 故不能用 ES module 的 `import().default`（IIFE 无 export）——改用动态注入
 * `<script src="/mermaid/mermaid.js">` 标签，加载后从 window 取 MermaidRenderer。
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

declare global {
  interface Window {
    MermaidRenderer?: MermaidApi;
  }
}

let mermaidPromise: Promise<MermaidApi> | null = null;

/**
 * 动态加载 mermaid 独立 IIFE bundle 的底层函数（可注入以便测试）。
 *
 * IIFE bundle 挂在 `window.MermaidRenderer`（非 ES module export），故用动态注入
 * `<script>` 标签的方式加载，标签 onload 后从 window 取。测试时可通过
 * [`_resetMermaidLoader`] 注入 mock。
 */
export let loadMermaidBundle: () => Promise<MermaidApi> = () =>
  new Promise((resolve, reject) => {
    // 已加载（同页多次调用 / SPA 导航）直接复用。
    if (window.MermaidRenderer) {
      resolve(window.MermaidRenderer);
      return;
    }
    const script = document.createElement('script');
    script.src = '/mermaid/mermaid.js';
    script.onload = () => {
      if (window.MermaidRenderer) {
        resolve(window.MermaidRenderer);
      } else {
        reject(new Error('mermaid bundle loaded but window.MermaidRenderer undefined'));
      }
    };
    script.onerror = () => reject(new Error('failed to load /mermaid/mermaid.js'));
    document.head.appendChild(script);
  });

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

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
 *
 * 主题切换：mermaid 把颜色/主题变量烤进 SVG 内联样式，无法靠 CSS 原地改主题。
 * 唯一办法是移除旧 SVG → 用新主题重新 render → 插入新 SVG。post_content.rs 的
 * use_effect 读 use_resolved_theme() 建立订阅，主题切换时重跑、重调 __initMermaid
 * 传入新 theme；本模块用 dataset.mermaidTheme 记住上次渲染主题，主题变化时触发重渲染。
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

/** render id 自增计数器，保证每次 render 生成唯一 id，避开 mermaid 残留节点冲突。 */
let renderCounter = 0;

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
 * 把 mermaid 源码渲染成 SVG 并注入父 <pre>。
 *
 * - render 用全局自增 id（`mermaid-svg-${++renderCounter}`），避免同页多次 render
 *   撞上 mermaid 内部残留的 `d`-前缀布局辅助节点（mermaid#357）与 marker id
 *   冲突（mermaid#5741）。
 * - 渲染前清空 pre.innerHTML，确保旧 SVG 与残留 `d-` 节点先消失再插入新 SVG，
 *   不让亮/暗两版并存。
 * - source 存进 dataset.mermaidSource，主题切换重渲染时回取（此时 <code> 已被
 *   SVG 替换，textContent 不再可用）。
 */
async function renderBlock(pre: HTMLPreElement, source: string, theme: ThemeName): Promise<void> {
  const mermaid = await loadMermaid();
  mermaid.initialize({
    startOnLoad: false,
    theme: theme === 'dark' ? 'dark' : 'default',
    securityLevel: 'strict',
  });
  const id = `mermaid-svg-${++renderCounter}`;
  const { svg } = await mermaid.render(id, source);
  // 清空 pre 旧内容（旧 SVG + mermaid 残留的 `d-` 辅助节点），再注入新 SVG。
  pre.innerHTML = svg;
  pre.dataset.mermaidRendered = 'true';
  pre.dataset.mermaidSource = source;
  pre.dataset.mermaidTheme = theme;
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
 * 两条互斥路径（渲染后 <code> 被 SVG 替换，故两选择器命中的 pre 不会重叠）：
 * 1. 仍含 `<code class="language-mermaid">` 的 pre（未渲染）→ 缓存源码 +
 *    IntersectionObserver 进视口懒加载。
 * 2. 已渲染（pre 内容是 SVG，dataset.mermaid-rendered 标记）→ 同主题幂等跳过
 *    （上下篇切换复用组件实例时 post_content.rs 的 effect 会重调本函数）；
 *    主题变化则取缓存源码重渲染（bundle 已加载，无需 IntersectionObserver）。
 *
 * @param selector 文章正文容器选择器（如 '.post-content'）
 * @param theme 当前生效主题，传给 mermaid 适配暗色
 */
export function initMermaid(selector: string, theme: ThemeName): void {
  const root = document.querySelector(selector);
  if (!root) return;

  // 路径 1：未渲染的块（<code> 还在）。
  const blocks = root.querySelectorAll<HTMLPreElement>('pre > code.language-mermaid');
  blocks.forEach((code) => {
    const pre = code.parentElement as HTMLPreElement | null;
    if (!pre) return;
    if (pre.dataset.mermaidRendered) return; // 理论不命中（renderBlock 同步替换 innerHTML），防御

    const source = code.textContent || '';
    pre.dataset.mermaidSource = source;
    observeBlock(pre, async () => {
      try {
        await renderBlock(pre, source, theme);
      } catch (err) {
        // 渲染失败（语法错误 / bundle 加载失败）：保留原始源码，加错误标记 class
        // 便于用户发现是 mermaid 源写错了。不破坏页面其余内容。
        console.error('mermaid render failed:', err);
        pre.classList.add('mermaid-error');
      }
    });
  });

  // 路径 2：已渲染的块（<code> 已被 SVG 替换，按 dataset 回找）。无条件执行，
  // 覆盖「页面上未渲染块与已渲染块并存」的场景。
  rerenderExistingBlocks(root, theme);
}

/**
 * 对已渲染（pre 内容已是 SVG、无 <code>）的块按缓存源码重渲染。
 *
 * 主题切换重跑 initMermaid 时，已渲染的 pre 里 <code> 已被 SVG 替换，
 * `pre > code.language-mermaid` 选择器不再命中。这里用 dataset 标记回找。
 */
function rerenderExistingBlocks(root: Element, theme: ThemeName): void {
  const rendered = root.querySelectorAll<HTMLPreElement>('pre[data-mermaid-rendered]');
  rendered.forEach((pre) => {
    if (pre.dataset.mermaidTheme === theme) return;
    const source = pre.dataset.mermaidSource;
    if (!source) return; // 无缓存源码无法重渲染，保守跳过
    void renderBlock(pre, source, theme).catch((err) => {
      console.error('mermaid re-render failed:', err);
      pre.classList.add('mermaid-error');
    });
  });
}

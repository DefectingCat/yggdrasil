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
import { onThemeChange } from './theme-transition';

/** 文章正文容器选择器（与 post_content.rs 的 __initMermaid 调用一致）。 */
const POST_CONTENT_SELECTOR = '.post-content';

type MermaidApi = {
  initialize: (config: Record<string, unknown>) => void;
  render: (id: string, text: string) => Promise<{ svg: string }>;
};

/**
 * Catppuccin themeVariables（与站点 --color-paper-* / .tmTheme 调色板对齐）。
 *
 * mermaid 把颜色烤进 SVG 内联 style，无法靠 CSS 原地改主题，故须在 initialize 时注入。
 * 用 `theme: 'base'`（非 'default'/'dark'）：base 主题不硬编码颜色，themeVariables 能完全
 * 控制调色板；'default' 主题硬编码 mainBkg=#ECECFF 等会阻断覆盖。
 *
 * 设计哲学：极简卡片化——节点用 surface 色阶（卡片感）、边框/连线用 subtext 色阶（克制）、
 * 文字用 primary text。不滥用强调色，绿/紫等 accent 留给作者用 classDef 手动强调。
 * hex 值取自 themes/Catppuccin Latte.tmTheme 与 Catppuccin Mocha.tmTheme。
 *
 * 覆盖的字段涵盖 flowchart / sequence / class 三类图（测试文章用到的全部类型）。
 */

/** Latte（亮）主题：节点 surface 色阶、文字 #4c4f69、连线 subtext1 #5c5f77。 */
const LATTE_VARS = {
  background: '#dce0e8', // = --color-paper-code-block，图背景与 pre 无缝衔接
  // 节点填充：surface 色阶递进（主/次/三级），卡片质感
  primaryColor: '#e6e9ef',
  secondaryColor: '#ccd0da',
  tertiaryColor: '#bcc0cc',
  mainBkg: '#e6e9ef',
  nodeBkg: '#e6e9ef',
  secondBkg: '#ccd0da',
  // 节点边框：surface1 偏冷灰
  primaryBorderColor: '#bcc0cc',
  secondaryBorderColor: '#acb0be',
  tertiaryBorderColor: '#9ca0b0',
  nodeBorder: '#bcc0cc',
  clusterBorder: '#bcc0cc',
  labelBoxBorderColor: '#bcc0cc',
  // 文字：primary text #4c4f69
  primaryTextColor: '#4c4f69',
  secondaryTextColor: '#5c5f77',
  tertiaryTextColor: '#6c6f85',
  textColor: '#4c4f69',
  nodeTextColor: '#4c4f69',
  titleColor: '#4c4f69',
  classText: '#4c4f69',
  labelTextColor: '#4c4f69',
  // 连线/箭头：subtext1 #5c5f77
  lineColor: '#5c5f77',
  defaultLinkColor: '#5c5f77',
  arrowheadColor: '#5c5f77',
  // 时序图
  actorBkg: '#e6e9ef',
  actorBorder: '#bcc0cc',
  actorTextColor: '#4c4f69',
  actorLineColor: '#5c5f77',
  signalColor: '#5c5f77',
  signalTextColor: '#4c4f69',
  loopTextColor: '#4c4f69',
  sequenceNumberColor: '#eff1f5',
  activationBkgColor: '#ccd0da',
  activationBorderColor: '#bcc0cc',
  // 边标签 / 子图背景
  edgeLabelBackground: '#eff1f5',
  labelBoxBkgColor: '#eff1f5',
  clusterBkg: 'rgba(239, 241, 245, 0.5)',
  // 注释：低饱和黄（Latte yellow #df8e1d）
  noteBkgColor: 'rgba(223, 142, 29, 0.15)',
  noteBorderColor: '#df8e1d',
  noteTextColor: '#4c4f69',
  // 字体：与正文 sans 对齐，中文友好
  fontFamily:
    "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans SC', 'PingFang SC', 'Microsoft YaHei', sans-serif",
  fontSize: '16px',
} as const;

/** Mocha（暗）主题：节点 surface 色阶、文字 #cdd6f4、连线 subtext0 #a6adc8。 */
const MOCHA_VARS = {
  background: '#313244', // = --color-paper-code-block（暗）
  primaryColor: '#45475a',
  secondaryColor: '#585b70',
  tertiaryColor: '#1e1e2e',
  mainBkg: '#45475a',
  nodeBkg: '#45475a',
  secondBkg: '#585b70',
  primaryBorderColor: '#585b70',
  secondaryBorderColor: '#45475a',
  tertiaryBorderColor: '#313244',
  nodeBorder: '#585b70',
  clusterBorder: '#585b70',
  labelBoxBorderColor: '#585b70',
  primaryTextColor: '#cdd6f4',
  secondaryTextColor: '#bac2de',
  tertiaryTextColor: '#a6adc8',
  textColor: '#cdd6f4',
  nodeTextColor: '#cdd6f4',
  titleColor: '#cdd6f4',
  classText: '#cdd6f4',
  labelTextColor: '#cdd6f4',
  lineColor: '#a6adc8',
  defaultLinkColor: '#a6adc8',
  arrowheadColor: '#a6adc8',
  actorBkg: '#45475a',
  actorBorder: '#585b70',
  actorTextColor: '#cdd6f4',
  actorLineColor: '#a6adc8',
  signalColor: '#a6adc8',
  signalTextColor: '#cdd6f4',
  loopTextColor: '#cdd6f4',
  sequenceNumberColor: '#1e1e2e',
  activationBkgColor: '#585b70',
  activationBorderColor: '#6c7086',
  edgeLabelBackground: '#1e1e2e',
  labelBoxBkgColor: '#1e1e2e',
  clusterBkg: 'rgba(30, 30, 46, 0.5)',
  noteBkgColor: 'rgba(249, 226, 175, 0.12)',
  noteBorderColor: '#f9e2af',
  noteTextColor: '#cdd6f4',
  fontFamily:
    "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans SC', 'PingFang SC', 'Microsoft YaHei', sans-serif",
  fontSize: '16px',
} as const;

/** 按主题返回对应 Catppuccin themeVariables。 */
function themeVarsFor(theme: ThemeName): Record<string, unknown> {
  return theme === 'dark' ? { ...MOCHA_VARS } : { ...LATTE_VARS };
}

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
    // base 主题不硬编码颜色，让 themeVariables 完全控制 Catppuccin 调色板。
    theme: 'base',
    darkMode: theme === 'dark',
    securityLevel: 'strict',
    // flowchart：平滑曲线 + 边缘留白 + 缩放至容器宽度（配合 CSS 消除横向滚动）。
    flowchart: {
      curve: 'basis',
      diagramPadding: 16,
      useMaxWidth: true,
      htmlLabels: true,
    },
    themeVariables: themeVarsFor(theme),
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
export function initMermaid(selector: string, theme: ThemeName): Promise<void> {
  const root = document.querySelector(selector);
  if (!root) return Promise.resolve();

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
  // 覆盖「页面上未渲染块与已渲染块并存」的场景。主题切换时由 onThemeChange 订阅
  // 返回其 Promise，供 VT callback await（让新主题流程图进入 NEW 快照）。
  return rerenderExistingBlocks(root, theme);
}

/**
 * 对已渲染（pre 内容已是 SVG、无 <code>）的块按缓存源码重渲染。
 *
 * 主题切换重跑 initMermaid 时，已渲染的 pre 里 <code> 已被 SVG 替换，
 * `pre > code.language-mermaid` 选择器不再命中。这里用 dataset 标记回找。
 *
 * 返回聚合 Promise：所有需要重渲染的块完成后 resolve。onThemeChange 订阅返回它，
 * VT callback await 它以等 mermaid.render 异步完成后再拍 NEW 快照。单块失败不中断
 * 聚合（catch 吞错 + 加 mermaid-error class）。
 */
function rerenderExistingBlocks(root: Element, theme: ThemeName): Promise<void> {
  const rendered = root.querySelectorAll<HTMLPreElement>('pre[data-mermaid-rendered]');
  const tasks: Promise<void>[] = [];
  rendered.forEach((pre) => {
    if (pre.dataset.mermaidTheme === theme) return;
    const source = pre.dataset.mermaidSource;
    if (!source) return; // 无缓存源码无法重渲染，保守跳过
    tasks.push(
      renderBlock(pre, source, theme).catch((err) => {
        console.error('mermaid re-render failed:', err);
        pre.classList.add('mermaid-error');
      }),
    );
  });
  return Promise.all(tasks).then(() => {});
}

/**
 * 订阅主题切换:主题变化时重渲染已渲染的 mermaid 块,返回重渲染 Promise。
 *
 * VT 协调的关键:本 listener 返回 Promise → notifyThemeChange 收集它 → VT callback
 * await 它 → 浏览器等 mermaid.render 完成、拍含新主题流程图的 NEW 快照、再播圆形扩散。
 * 无 VT(降级 / 跟随系统瞬切)时调用方不等,mermaid 后台重渲染。
 *
 * 顶层注册(IIFE 加载时),确保任何时刻主题切换都能命中。首次渲染前无已渲染块,
 * rerenderExistingBlocks 自然 no-op。
 */
onThemeChange((isDark) => {
  const root = document.querySelector(POST_CONTENT_SELECTOR);
  if (!root) return;
  return rerenderExistingBlocks(root, isDark ? 'dark' : 'light');
});

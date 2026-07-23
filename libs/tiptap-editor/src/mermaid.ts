import { mermaidThemeVarsFor, type ThemeName } from '@yggdrasil/shared';

/**
 * 编辑器侧 mermaid 加载与渲染封装。
 *
 * 与前台 yggdrasil-core/src/mermaid.ts 共用同一份运行时(`/mermaid/mermaid.js`,
 * mermaid 11.16.0,挂 window.MermaidRenderer)与同一套 Catppuccin 主题变量
 * (经 @yggdrasil/shared 单一真相源),保证「编辑器预览 = 线上文章页」。
 *
 * 区别于前台:编辑器是 NodeView 内的同步生命周期里发起异步渲染,需由调用方
 * (CodeBlockNodeView)管理 debounce、竞态取消与主题重渲染。本模块只提供
 * 纯粹的「加载 bundle + 把源码渲染成 SVG/error」能力。
 */

/** mermaid 11 API 子集(项目只用 initialize + render)。 */
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
let renderCounter = 0;

/**
 * 动态加载 mermaid 独立 IIFE bundle(`/mermaid/mermaid.js`)。
 *
 * bundle 挂 window.MermaidRenderer(IIFE 无 ES export),故用动态注入 <script>
 * 标签的方式加载,onload 后从 window 取。单例缓存,失败清空允许重试。
 * 与前台 mermaid.ts:201-209 同逻辑。
 */
export function loadMermaidRenderer(): Promise<MermaidApi> {
  if (!mermaidPromise) {
    mermaidPromise = new Promise<MermaidApi>((resolve, reject) => {
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
    }).catch((err) => {
      // 失败清空缓存,允许下次重试。
      mermaidPromise = null;
      throw err;
    });
  }
  return mermaidPromise;
}

/** 测试用:重置加载缓存(注入 mock 后必须调)。 */
export function _resetMermaidRendererLoader(): void {
  mermaidPromise = null;
}

/**
 * 把 mermaid 源码渲染成 SVG。
 *
 * 渲染配置与前台 mermaid.ts:224-238 完全一致:theme:'base' 让 themeVariables
 * 完全控制调色板、securityLevel:'strict'、flowchart 平滑曲线 + useMaxWidth。
 * initialize 每次渲染前调(前台同款),保证主题变量即时生效。
 *
 * @returns 成功返回 { svg },失败返回 { error }(不抛异常,调用方据此显示错误态)。
 */
export async function renderMermaid(
  source: string,
  theme: ThemeName,
): Promise<{ svg: string } | { error: string }> {
  try {
    const mermaid = await loadMermaidRenderer();
    mermaid.initialize({
      startOnLoad: false,
      theme: 'base',
      darkMode: theme === 'dark',
      securityLevel: 'strict',
      flowchart: {
        curve: 'basis',
        diagramPadding: 16,
        useMaxWidth: true,
        htmlLabels: true,
      },
      themeVariables: mermaidThemeVarsFor(theme),
    });
    const id = `tiptap-mermaid-${++renderCounter}`;
    const { svg } = await mermaid.render(id, source);
    return { svg };
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return { error: msg };
  }
}

/**
 * 读取当前站点主题(前台用 documentElement 的 .dark class 标记暗色)。
 * NodeView 首次渲染与主题切换重渲染时调用。
 */
export function getCurrentTheme(): ThemeName {
  return document.documentElement.classList.contains('dark') ? 'dark' : 'light';
}

/**
 * mermaid 测试：钉住懒加载渲染的扫描、幂等、主题适配与错误回退。
 * 黑盒驱动：通过 window.__initMermaid 入口，mock IntersectionObserver 与 mermaid bundle 加载。
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import './index';
import { _resetMermaidLoader } from './mermaid';

// mock IntersectionObserver：observe 时立即异步触发 isIntersecting 回调，模拟块进视口。
const disconnect = vi.fn();
const observe = vi.fn();
let intersectCallback: ((entries: { isIntersecting: boolean }[]) => void) | null = null;
vi.stubGlobal(
  'IntersectionObserver',
  class {
    constructor(cb: (entries: { isIntersecting: boolean }[]) => void) {
      intersectCallback = cb;
    }
    observe() {
      observe();
      // 立即触发可见，模拟块已在视口内。
      if (intersectCallback) intersectCallback([{ isIntersecting: true }]);
    }
    disconnect() {
      disconnect();
    }
  },
);

describe('initMermaid', () => {
  const mockRender = vi.fn().mockResolvedValue({ svg: '<svg>diagram</svg>' });
  const mockInitialize = vi.fn();

  beforeEach(() => {
    document.body.innerHTML = '';
    mockRender.mockResolvedValue({ svg: '<svg>diagram</svg>' });
    mockRender.mockClear();
    mockInitialize.mockClear();
    observe.mockClear();
    disconnect.mockClear();
    // 注入 mock mermaid bundle 加载函数
    _resetMermaidLoader(async () => ({ initialize: mockInitialize, render: mockRender }));
  });
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('扫描 language-mermaid 块并渲染成 SVG', async () => {
    const root = document.createElement('div');
    root.className = 'post-content';
    root.innerHTML = '<pre><code class="language-mermaid">graph TD; A--&gt;B</code></pre>';
    document.body.appendChild(root);

    window.__initMermaid('.post-content', 'light');

    await vi.waitFor(() => {
      expect(root.querySelector('pre')?.innerHTML).toContain('<svg>diagram</svg>');
    });
  });

  it('用 dark 主题初始化 mermaid', async () => {
    const root = document.createElement('div');
    root.className = 'post-content';
    root.innerHTML = '<pre><code class="language-mermaid">graph TD; A--&gt;B</code></pre>';
    document.body.appendChild(root);

    window.__initMermaid('.post-content', 'dark');

    await vi.waitFor(() => {
      expect(mockInitialize).toHaveBeenCalledWith(expect.objectContaining({ theme: 'dark' }));
    });
  });

  it('幂等：重复调用不重复渲染已处理的块', async () => {
    const root = document.createElement('div');
    root.className = 'post-content';
    root.innerHTML = '<pre><code class="language-mermaid">graph TD; A--&gt;B</code></pre>';
    document.body.appendChild(root);

    window.__initMermaid('.post-content', 'light');
    await vi.waitFor(() => {
      expect(root.querySelector('pre')?.dataset.mermaidRendered).toBe('true');
    });
    mockRender.mockClear();

    // 第二次调用（模拟上下篇切换）：不应再次 render
    window.__initMermaid('.post-content', 'light');
    await new Promise((r) => setTimeout(r, 50));
    expect(mockRender).not.toHaveBeenCalled();
  });

  it('非 mermaid 代码块不受影响', () => {
    const root = document.createElement('div');
    root.className = 'post-content';
    root.innerHTML = '<pre><code class="language-rust">fn main() {}</code></pre>';
    document.body.appendChild(root);

    expect(() => window.__initMermaid('.post-content', 'light')).not.toThrow();
    // 不应尝试渲染 rust 块
    expect(observe).not.toHaveBeenCalled();
  });

  it('selector 未命中时不报错', () => {
    expect(() => window.__initMermaid('.not-exist', 'light')).not.toThrow();
  });

  it('渲染失败时加 mermaid-error class', async () => {
    mockRender.mockRejectedValueOnce(new Error('syntax error'));
    const root = document.createElement('div');
    root.className = 'post-content';
    root.innerHTML = '<pre><code class="language-mermaid">bad syntax</code></pre>';
    document.body.appendChild(root);

    window.__initMermaid('.post-content', 'light');

    await vi.waitFor(() => {
      expect(root.querySelector('pre')?.classList.contains('mermaid-error')).toBe(true);
    });
  });

  it('主题切换时重渲染已渲染的块', async () => {
    const root = document.createElement('div');
    root.className = 'post-content';
    root.innerHTML = '<pre><code class="language-mermaid">graph TD; A--&gt;B</code></pre>';
    document.body.appendChild(root);

    // 首次渲染（light）
    window.__initMermaid('.post-content', 'light');
    await vi.waitFor(() => {
      expect(root.querySelector('pre')?.dataset.mermaidRendered).toBe('true');
    });
    expect(root.querySelector('pre')?.dataset.mermaidTheme).toBe('light');
    const firstRenderCalls = mockRender.mock.calls.length;

    // 主题切换 → dark：应触发重渲染
    window.__initMermaid('.post-content', 'dark');
    await vi.waitFor(() => {
      expect(mockRender.mock.calls.length).toBeGreaterThan(firstRenderCalls);
    });
    expect(mockInitialize).toHaveBeenLastCalledWith(expect.objectContaining({ theme: 'dark' }));
    expect(root.querySelector('pre')?.dataset.mermaidTheme).toBe('dark');
  });

  it('主题未变时重渲染路径幂等（同主题跳过）', async () => {
    const root = document.createElement('div');
    root.className = 'post-content';
    root.innerHTML = '<pre><code class="language-mermaid">graph TD; A--&gt;B</code></pre>';
    document.body.appendChild(root);

    window.__initMermaid('.post-content', 'light');
    await vi.waitFor(() => {
      expect(root.querySelector('pre')?.dataset.mermaidRendered).toBe('true');
    });
    mockRender.mockClear();

    // 同主题再调（模拟上下篇切换复用组件实例、effect 重跑）
    window.__initMermaid('.post-content', 'light');
    await new Promise((r) => setTimeout(r, 50));
    expect(mockRender).not.toHaveBeenCalled();
  });

  it('主题切换重渲染用唯一 render id（避免 mermaid 残留节点冲突）', async () => {
    const root = document.createElement('div');
    root.className = 'post-content';
    root.innerHTML = '<pre><code class="language-mermaid">graph TD; A--&gt;B</code></pre>';
    document.body.appendChild(root);

    window.__initMermaid('.post-content', 'light');
    await vi.waitFor(() => {
      expect(root.querySelector('pre')?.dataset.mermaidRendered).toBe('true');
    });
    const firstId = mockRender.mock.calls[0][0];

    window.__initMermaid('.post-content', 'dark');
    await vi.waitFor(() => {
      expect(mockRender.mock.calls.length).toBeGreaterThanOrEqual(2);
    });
    const secondId = mockRender.mock.calls[mockRender.mock.calls.length - 1][0];

    // 两次 render 的 id 必须不同，否则撞上 mermaid 内部残留的 d-前缀节点（#357）
    expect(secondId).not.toBe(firstId);
  });
});

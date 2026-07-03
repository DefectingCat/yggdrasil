---
name: yggdrasil-ui-design-taste
description: |
  Yggdrasil 项目的专属 UI 设计规范与组件审美风格指南。
  指导 AI 遵循项目最新的“现代化极简”与“卡片化”设计语言：
  包含超大圆角（2rem / 32px）、无感阴影边框（box-shadow）、
  毛玻璃浮动导航栏、响应式表格单元格圆角、以及基于组件挂载（key-based mount）的路由切换动画，
  避免骨架屏截断动画等反模式。
metadata:
  trigger: 编写或修改 CSS、input.css、主前台页面（src/pages/）、主布局（src/components/frontend_layout.rs）以及 Markdown 渲染表格等
  source: 提炼自项目前台极简设计重构与动画渲染问题修复
---

# Yggdrasil 现代化极简与卡片化 UI 设计规范

本规范定义了 Yggdrasil 项目前台页面的核心视觉特征、设计系统 Token、组件样式标准以及动画机制。修改任何前台 UI、样式及相关组件时，必须严格遵守本设计语言，避免任何粗糙、不协调的 AI 默认样式。

## 核心设计哲学
1. **去实体边框化**：摒弃粗硬的 1px 实线边框，改用轻盈无感的阴影（如 `box-shadow: 0 0 0 1px var(--color-paper-border)`）或浮动卡片背景。
2. **大呼吸感圆角**：卡片和图片封面圆角统一为 `2rem` (`32px`)，以形成圆润饱满的现代视觉张力。
3. **Catppuccin 语义配色**：基于 Catppuccin Latte/Mocha 调色，仅允许最大一个不饱和的强调色 `var(--color-paper-accent)`。
4. **纯净流式的响应式排版**：内容区严格采用 max-width 限制，表头字号随视口尺寸 `clamp()` 响应收紧，不使用多余的修饰性元素。

---

## 核心视觉组件规范

### 1. 文章卡片布局（Post Card）
* **完美对称内边距**：文字区域包裹的 `div` 必须使用统一的 `p-8` (32px) 的内边距。不管有没有封面图片，文字四周的呼吸感应完全对称。
* **卡片封面**：大圆角与外侧卡片齐平，比例固定为 `21:9` 或 `4:3`。必须配合 `blur-img` 结构实现渐进式模糊加载。
* **悬浮动态反馈**：当卡片被 Hover 时，封面图进行轻微缓慢的缩放（`.group:hover .post-card-cover-blur img` 使用 `transform: scale(1.05)` 配合 0.3s transition），并且卡片文字可以通过 `.post-card-accent` 下划线滑过进行视觉确认。

### 2. 浮动毛玻璃导航栏（Navbar）
* **悬浮质感**：不设底边线，采用浮动在页面上方的毛玻璃设计。
* **样式类**：包含 `backdrop-blur` 高强度模糊、高透明底色、充盈的内边距，确保背景内容滚过时若隐若现，增加层次感。

### 3. 极简现代表格（Markdown Table）
Markdown 渲染出来的表格默认很难看，本项目使用专门优化的 CSS 类。编写/修改表格样式或渲染逻辑时需遵循：
* **结构与对齐**：使用 `border-collapse: separate` 和 `border-spacing: 0`。**千万不要**对 `table` 本身使用 `display: block`，否则会破坏表格自带的对齐拉伸逻辑。
* **防背景溢出圆角（关键像素级处理）**：由于 `display: table` 无法被外层 `overflow: hidden` 完美截断，必须给四个角落的单元格设置对应的 `border-radius: 16px`，防止 `th` 的背景溢出大圆角：
  - 左上：`table thead th:first-child { border-top-left-radius: 16px; }`
  - 右上：`table thead th:last-child { border-top-right-radius: 16px; }`
  - 左下：`table tbody tr:last-child td:first-child { border-bottom-left-radius: 16px; }`
  - 右下：`table tbody tr:last-child td:last-child { border-bottom-right-radius: 16px; }`
* **无感边框与配色**：
  - 使用 `box-shadow: 0 0 0 1px var(--color-paper-border)` 代替 `border`。
  - 表头 (`th`) 使用 `var(--color-paper-code-block)` 软灰色底。
  - 行间分割线为 `1px solid var(--color-paper-border)`，最后一行无下分割线。
  - 悬浮时整行高亮 `background: var(--color-paper-entry)`。

---

## 路由页面过渡动画规范

项目前台采用 WebAssembly 异步获取数据结构，页面切换时有骨架屏挂载。若动画配置不当，会被骨架屏“截断”导致真实页面显示时没有过渡。

### 1. 动画定义
在 `input.css` 中定义的平滑进场动画类 `.animate-page-enter`：
```css
@keyframes page-enter {
  0% {
    opacity: 0;
    transform: translateY(16px) scale(0.995);
  }
  100% {
    opacity: 1;
    transform: none;
  }
}
.animate-page-enter {
  animation: page-enter 400ms cubic-bezier(0.22, 1, 0.36, 1) both;
}
```

### 2. 挂载时机（反模式避坑）
* ❌ **错误的做法**：在 `frontend_layout.rs` 里的外层 `main` 或 `Outlet` 容器加上 `.animate-page-enter`。这会导致数据尚未就绪时骨架屏动画已播完，真实内容闪现出来。
* ✅ **正确的做法**：直接将 `.animate-page-enter` 加在**每个具体页面组件（如 Home, Archives, Tags, PostDetail, About 等）的真实渲染内容的最外层 `div`（或 `article`）上**。
  - 例（`about.rs`）：
    ```rust
    pub fn About() -> Element {
        rsx! {
            div { class: "animate-page-enter",
                // 页面具体真实内容...
            }
        }
    }
    ```
  这确保当且仅当异步数据加载完毕、真实的 DOM 元素挂载（mount）到浏览器时，页面才独立触发一次平滑的淡入上滑进场。

---

## 违规自检清单
在提交前台 UI 修改前，请务必核对：
- [ ] 卡片内边距是否为统一的 `p-8`，是否有四周不对称的边距？
- [ ] 表格是否被设置成了 `display: block` 导致对齐失效？
- [ ] 表格的四个角落是否用 `border-*-radius` 对齐，是否存在表头直角溢出？
- [ ] 页面动画是否挂载到了具体页面组件的真实内容节点？是否会被骨架屏抢走？
- [ ] 颜色是否全部使用 Catppuccin 语义 token？是否引入了过于鲜艳的 AI 渐变？

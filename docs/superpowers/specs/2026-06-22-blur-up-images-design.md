# 文章图片 Blur-up 渐进加载设计

## 背景与目标

文章页面的缩略图（卡片封面 `?thumb=400x300`、详情封面 `?w=1200`、正文图 `?w=800`）在加载过程中**没有任何占位**：`<img>` 裸标签无 width/height/aspect-ratio，加载时区域高度为 0，加载完撑开——导致 CLS（布局跳动）明显，尤其在列表页和长文章里。

本设计的目标：
- 加载前先显示**低分辨率模糊占位图**（`?w=20`），消除空白与跳动
- 加载完高清缩略图后，**平滑过渡**到清晰（opacity 淡入）
- 服务端 SSR 内嵌占位图（渐进增强，JS 禁用也能看到占位）
- 三处图片位置（卡片封面、详情封面、正文图）统一应用

## 关键决策

| 决策点 | 选择 |
|--------|------|
| 占位图来源 | SSR 内嵌（HTML 里 img src 就是占位图 URL，JS 替换高清） |
| 过渡实现 | 双层叠加 + opacity 淡入（底层占位图常驻，上层高清图淡入覆盖） |
| 作用范围 | 三处都加（卡片封面、详情封面、正文图） |
| 占位图尺寸 | 统一 `?w=20` |
| DOM 生成方式 | 方案 A：SSR 渲染双层结构 + JS 接管高清加载 |
| aspect-ratio 尺寸来源 | 方案 B2：渲染时实时读图片 header + moka 缓存（零迁移） |
| 外链图 | 不处理（保持原生 img，无法生成占位图） |
| dimensions 缓存 TTL | 24h，可通过 `IMAGE_DIMENSIONS_CACHE_TTL_SECS` 环境变量配置 |

## 技术约束（探索结论）

- **服务端图片能力现成**：`/uploads/{path}` 支持 `?w=`/`?thumb=`，已有内存+磁盘两级缓存（`src/api/image.rs`）。`?w=20` 产出 <1KB 的极小图。
- **sanitizer 约束**：当前 `clean_post_html` 对 img 只放行 `src/alt/width/height/align`，对 span 放行 `class`。双层结构需要额外放行 `data-src`/`class`/`style`（img）和 `style`（span）。评论配置不动。
- **现有缓存是异步的**：`src/cache.rs` 用 `moka::future::Cache`（`.get().await`），但 `render_markdown_enhanced` 是同步函数。dimensions 缓存需要用 `moka::sync::Cache`（同步版本）。
- **webp 尺寸读取可行**：`zenwebp::WebPDecoder::build(data)` + `.info()` 只解析 WebP header（RIFF chunk）就能拿到 width/height，**不需要全量解码像素**（见 `src/webp.rs:143-145` 现有代码在 decode 前就读了 info）。开销极小。GIF/PNG/JPEG 走 `image::ImageReader::into_dimensions()`（只读 header）。
- **上传后磁盘格式**：非 gif/webp 统一转 webp，gif 保持 gif，webp 保持 webp。所以 dimensions 读取要支持 webp + gif/png/jpeg。

## 详细设计

### 架构总览

```
SSR 渲染文章/封面时
  ↓
对每张 /uploads/ 图片:
  ├─ get_image_dimensions(path) → (w, h)  [sync moka cache + 读 header]
  └─ 产出双层 DOM:
      <span class="blur-img" style="--ar: W/H;">
        <img class="blur-img-placeholder" src="...?w=20">   ← SSR 内嵌,立即加载
        <img class="blur-img-full" data-src="...?w=800">    ← JS 懒加载
      </span>

前端 JS (post-content.js / image-viewer 初始化)
  ├─ IntersectionObserver 监听 .blur-img 进入视口
  ├─ 把 .blur-img-full 的 data-src 赋给 src → 触发高清图加载
  └─ 高清图 onload → 加 .is-loaded class → CSS opacity 0→1 淡入
```

### 1. Dimensions 缓存与读取（新增）

**`src/api/image.rs` 新增**（`#[cfg(feature = "server")]`）：

```rust
use moka::sync::Cache;
use std::sync::LazyLock;
use std::time::Duration;

static IMAGE_DIMENSIONS_CACHE: LazyLock<Cache<String, (u32, u32)>> = LazyLock::new(|| {
    let ttl = std::env::var("IMAGE_DIMENSIONS_CACHE_TTL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(86400)); // 默认 24h
    Cache::builder().time_to_live(ttl).build()
});

/// 读取图片真实尺寸（只读 header，不解码像素）。
/// 路径如 "2026/06/22/xxx.webp"（不含 /uploads/ 前缀）。
/// 失败返回 None（渲染时回退到不设 aspect-ratio）。
pub fn get_image_dimensions(rel_path: &str) -> Option<(u32, u32)> {
    if let Some(dims) = IMAGE_DIMENSIONS_CACHE.get(rel_path) {
        return Some(dims);
    }
    let full_path = std::path::Path::new("uploads").join(rel_path);
    let data = std::fs::read(&full_path).ok()?;
    let dims = read_dimensions_from_bytes(&data, rel_path)?;
    IMAGE_DIMENSIONS_CACHE.insert(rel_path.to_string(), dims);
    Some(dims)
}

/// 按扩展名分发：webp 走 zenwebp header，其他走 image crate。
fn read_dimensions_from_bytes(data: &[u8], path: &str) -> Option<(u32, u32)> {
    let ext = std::path::Path::new(path).extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "webp" => {
            let decoder = zenwebp::WebPDecoder::build(data).ok()?;
            let info = decoder.info();
            Some((info.width, info.height))
        }
        "gif" | "png" | "jpg" | "jpeg" => {
            let reader = image::ImageReader::new(std::io::Cursor::new(data))
                .with_guessed_format().ok()?;
            reader.into_dimensions().ok()
        }
        _ => None,
    }
}
```

**注意**：这是同步函数，在 server function 的异步上下文里同步读文件头。文件头很小（几十到几百字节），开销可接受；moka cache 命中时不读盘。`std::fs::read` 读整个文件——优化点：可改为只读前 N 字节（webp/jpeg/png/gif 的 header 都在前几百字节内），但首版用 `std::fs::read` 保持简单，配合 cache 降低频率。

### 2. 双层 DOM 结构

```html
<span class="blur-img" style="--ar: 16/9;">
  <img class="blur-img-placeholder" src="/uploads/x.webp?w=20" alt="...">
  <img class="blur-img-full" data-src="/uploads/x.webp?w=800" alt="...">
</span>
```

- **`span.blur-img`**：相对定位容器，`aspect-ratio: var(--ar)` 预留空间（零 CLS），`overflow: hidden` 裁剪模糊边缘
- **`img.blur-img-placeholder`**：SSR 内嵌的 `?w=20` 占位图，绝对定位填满容器，CSS 模糊放大
- **`img.blur-img-full`**：`data-src` 存高清 URL，初始无 src（不立即加载），JS 接管；绝对定位，初始 `opacity: 0`，加载完 `.is-loaded` → `opacity: 1`

`--ar` 由服务端用 `get_image_dimensions` 拿到的真实宽高生成（如 `16/9`）。dimensions 读取失败时不设 `--ar`，退化为无 aspect-ratio（轻微 CLS，但不阻塞渲染）。

### 3. Markdown 渲染器改造（`render_markdown_enhanced`）

正文图转换发生在 Markdown→HTML 阶段。当前用 `pulldown_cmark::html::push_html` 产出 `<img src="..." alt="...">`。

**改造**：在 `push_html` 产出 HTML 后、`clean_html` 之前，对字符串里的 `<img>` 做后处理：

```rust
fn wrap_images_with_blur(html: &str) -> String {
    // 用正则/简单解析找到 <img src="/uploads/..." ...> 标签
    // 对每个匹配的 img:
    //   1. 提取 src（如 /uploads/2026/06/22/x.webp）
    //   2. 从 src 解析出 rel_path（去掉 /uploads/ 前缀和 query）
    //   3. get_image_dimensions(rel_path) → (w,h)，算 --ar
    //   4. 生成双层结构替换原 img
    // 非 /uploads/ 的外链图不处理（保持原样）
}
```

**正则选择**：用 `regex` crate（已是 server feature 的 optional 依赖）匹配 `<img\s+[^>]*src="(/uploads/[^"?]+)[^"]*"[^>]*>`。注意 HTML 用正则有边界情况，但 img 标签由 pulldown-cmark 生成，格式可控（属性顺序固定：src 在前，alt 在后）。

### 4. ImageViewer 改造（卡片封面 / 详情封面）

`ImageViewer` 当前渲染单层 img + 灯箱。改为渲染双层结构：

```rust
ImageViewer {
    src: cover.clone(),
    thumb_params: "?w=1200",        // 高清尺寸 → data-src
    placeholder_params: "?w=20",    // 占位图尺寸
    alt: "封面图片",
}
```

ImageViewer 内部：
- **dimensions 获取**：组件内部调用 `get_image_dimensions`（`#[cfg(feature = "server")]`）。SSR 渲染时组件树在服务端执行，server-only 函数可用；dimensions 有 moka 缓存，命中时是内存查询，miss 时读小文件头。调用方（`post_card.rs`/`post_cover.rs`）无需改动 dimensions 相关逻辑。WASM 端无法调此函数，但图片的 `--ar` 在 SSR 阶段已渲染进 HTML，前端无需再算。
- 底层 placeholder：`src = {原图}{placeholder_params}`
- 上层 full：`data-src = {原图}{thumb_params}`
- `--ar` 由 dimensions 计算，SSR 时写入 `style`
- 灯箱逻辑不变（点击用原图 src 放大）

`get_image_dimensions` 在组件里调用属于 SSR 渲染时的同步读盘。首版接受这个阻塞（小文件 + cache）；如 SSR 性能成问题，后续可改为 server function 预算 + 模型字段传递。

### 5. post-content.js 改造

当前职责：把 img.src 改成 `?w=800`、加灯箱。

**新职责**：
- **不再改 src**（渲染器已产出双层结构，placeholder src 已是 `?w=20`）
- **加载高清图**：用 `IntersectionObserver` 监听 `.blur-img` 进入视口，把 `.blur-img-full` 的 `data-src` 赋给 `src`
- **触发淡入**：高清 img 的 `onload` 加 `.is-loaded` class，CSS 控制 opacity 过渡
- **灯箱**：保留现有点击放大。灯箱用 `.blur-img-full` 的 `data-src`（高清图 URL）作为放大图源——灯箱是看大图，应用高清版本而非 20px 占位图。

```javascript
function initBlurUp(root) {
  var containers = root.querySelectorAll('.blur-img');
  if (!('IntersectionObserver' in window)) {
    // 不支持 IO 的浏览器：直接加载所有
    containers.forEach(loadFull);
    return;
  }
  var io = new IntersectionObserver(function(entries) {
    entries.forEach(function(entry) {
      if (entry.isIntersecting) {
        loadFull(entry.target);
        io.unobserve(entry.target);
      }
    });
  }, { rootMargin: '200px' });
  containers.forEach(function(c) { io.observe(c); });
}

function loadFull(container) {
  var full = container.querySelector('.blur-img-full');
  if (!full || !full.dataset.src) return;
  full.onload = function() { full.classList.add('is-loaded'); };
  full.src = full.dataset.src;
}
```

### 6. CSS

新增到全局样式（`input.css` 或对应样式入口）：

```css
.blur-img {
  position: relative;
  display: block;
  overflow: hidden;
  aspect-ratio: var(--ar);
  background: var(--color-paper-code-bg, #f5f5f5); /* 加载中灰底 */
}
.blur-img-placeholder {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  filter: blur(20px) saturate(1.2);
  transform: scale(1.1); /* 放大遮住 blur 边缘 */
}
.blur-img-full {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0;
  transition: opacity 0.4s ease;
  z-index: 1;
}
.blur-img-full.is-loaded {
  opacity: 1;
}
```

暗色模式：`.blur-img` 的 background 用暗色灰底。

### 7. sanitizer 配置扩展

`clean_post_html`（文章专用）扩展放行属性：
- img：额外放行 `data-src`、`class`、`style`
- span：额外放行 `style`（用于 `--ar`）

评论配置（`clean_comment_html`）**不动**（评论无 img）。

## 实现边界与清单

### 服务端（`#[cfg(feature = "server")]`）

| 文件 | 改动 |
|------|------|
| `src/api/image.rs` | 新增 `IMAGE_DIMENSIONS_CACHE`（moka sync）+ `get_image_dimensions` + `read_dimensions_from_bytes` |
| `src/api/markdown.rs` | `render_markdown_enhanced` 增加 `wrap_images_with_blur` 后处理（在 clean_html 前） |
| `src/api/sanitizer.rs` | `clean_post_html` 扩展 img 放行 `data-src/class/style`、span 放行 `style` |

### 前端（Rust 组件）

| 文件 | 改动 |
|------|------|
| `src/components/image_viewer.rs` | 渲染双层结构（placeholder + full），接收 dimensions/aspect-ratio |
| `src/components/post_card.rs` | 调 ImageViewer 时传入 dimensions（SSR 算好） |
| `src/components/post/post_cover.rs` | 同上 |

### 前端（JS）

| 文件 | 改动 |
|------|------|
| `public/js/post-content.js` | 删除改 src 逻辑；新增 IntersectionObserver 懒加载 + onload 淡入；灯箱改用 data-src |

### 样式

| 文件 | 改动 |
|------|------|
| `input.css`（或全局样式入口） | 新增 `.blur-img*` 样式（含暗色模式） |

### 配置

| 文件 | 改动 |
|------|------|
| `.env.example` | 新增 `IMAGE_DIMENSIONS_CACHE_TTL_SECS=86400` |

### 不做的事

- 不改评论图片（评论无 img）
- 不处理外链图（非 `/uploads/` 的图保持原生 img）
- 不给上传流程加尺寸持久化（B2 方案零迁移，实时读 + 缓存）
- 不做 LQIP 之外的图片优化（avif 转码、响应式 srcset 等）
- 不改图片服务端点（`/uploads/{path}` 的 `?w=` 能力现成）

## 实现风险

1. **正则解析 img 标签**：`wrap_images_with_blur` 用正则匹配 img。pulldown-cmark 产出的 img 格式可控（src 在前、alt 在后），但需写测试覆盖各种 src 格式（带/不带 query、外链、相对路径）。

2. **`std::fs::read` 读全文件**：首版 `get_image_dimensions` 读整个文件再解析 header。对大图（几 MB）有内存/IO 开销。优化：改为只读前 N 字节（webp/jpeg/png/gif header 都在前 100 字节内）。首版保持简单，配合 cache 降低频率，后续可优化。

3. **SSR 阶段 ImageViewer 拿 dimensions**：ImageViewer 是通用组件，dimensions 是 server-only。需在 SSR 上层（post_card/post_cover）算好传入，避免组件内部调用 server-only 函数。

## 验收标准

- [ ] 文章正文图片加载时先显示模糊的 `?w=20` 占位图，无布局跳动（aspect-ratio 预留空间）
- [ ] 高清缩略图加载完后，opacity 平滑淡入覆盖占位图（0.4s 过渡）
- [ ] 视口外的高清图不加载（IntersectionObserver 懒加载）
- [ ] 卡片封面、详情封面同样有 blur-up 效果
- [ ] 外链图（非 /uploads/）保持原生 img，无 blur-up
- [ ] JS 禁用时，正文图显示 `?w=20` 模糊占位图（渐进增强，可读但模糊）
- [ ] dimensions 缓存命中时不读盘（二次访问快）
- [ ] webp/gif/png/jpeg 图片的尺寸都能正确读取
- [ ] `.env.example` 含 `IMAGE_DIMENSIONS_CACHE_TTL_SECS=86400`
- [ ] sanitizer 不误删双层结构的 `data-src`/`class`/`style` 属性
- [ ] 评论图片不受影响（评论无 img，配置不动）
- [ ] 点击图片放大（灯箱）功能仍正常工作

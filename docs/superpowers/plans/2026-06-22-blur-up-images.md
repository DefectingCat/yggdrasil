# 文章图片 Blur-up 渐进加载 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 文章页面图片加载时显示低分辨率模糊占位图，高清缩略图加载完后平滑淡入，消除 CLS。

**Architecture:** SSR 渲染双层 DOM（底层 `?w=20` 模糊占位 + 上层高清 `data-src`），服务端读图片 header 拿真实尺寸生成 aspect-ratio（moka sync cache），前端 JS 用 IntersectionObserver 懒加载高清图并淡入。

**Tech Stack:** Rust（image crate + zenwebp + moka sync cache + regex）、Dioxus 组件、原生 JS（IntersectionObserver）、CSS

**前置 spec:** `docs/superpowers/specs/2026-06-22-blur-up-images-design.md`

**关键约定（贯穿所有任务）：**
- 仅处理 `/uploads/` 路径的图片；外链图保持原生 img
- dimensions 读取是 server-only（`#[cfg(feature = "server")]`），用 `moka::sync::Cache`
- 正则匹配 img 标签由 pulldown-cmark 产出，格式可控（src 在前、alt 在后）
- 涉及 `#[cfg(target_arch = "wasm32")]` 或 `#[cfg(feature = "server")]` 的代码改动，验证必须用 `dx check`（wasm32 target）+ `cargo test`（默认 target），不能只靠 `cargo check`

---

## 文件结构

| 文件 | 责任 | 操作 |
|------|------|------|
| `src/api/image.rs` | `IMAGE_DIMENSIONS_CACHE`（moka sync）+ `get_image_dimensions` + `read_dimensions_from_bytes` | 改（新增） |
| `src/api/sanitizer.rs` | `clean_html` 扩展 img 放行 `data-src/class/style`、span 放行 `style` | 改 |
| `src/api/markdown.rs` | `render_markdown_enhanced` 增加 `wrap_images_with_blur` 后处理 | 改 |
| `src/components/image_viewer.rs` | 渲染双层结构（placeholder + full） | 改 |
| `public/js/post-content.js` | 删除改 src；新增 IntersectionObserver 懒加载 + onload 淡入 | 改 |
| `input.css` | 新增 `.blur-img*` 样式（含暗色） | 改 |
| `.env.example` | 新增 `IMAGE_DIMENSIONS_CACHE_TTL_SECS=86400` | 改 |

任务顺序：1（dimensions cache）→ 2（sanitizer）→ 3（markdown 包装）→ 4（CSS）→ 5（ImageViewer）→ 6（post-content.js）→ 7（.env）→ 8（验证）。

---

## Task 1: Dimensions 缓存与读取（image.rs）

**Files:**
- Modify: `src/api/image.rs`（顶部 import 区 + 文件末尾新增）

服务端读图片真实尺寸（只读 header），moka sync cache 缓存。这是后续任务的基础。

- [ ] **Step 1: 添加 moka::sync::Cache import**

在 `src/api/image.rs` 的 `#[cfg(feature = "server")] use moka::future::Cache;`（约第 17 行）之后添加：

```rust
#[cfg(feature = "server")]
use moka::sync::Cache as SyncCache;
```

注意：保留原有 `moka::future::Cache`（图片处理管线还在用），新增 `SyncCache` 别名给 dimensions 用。

- [ ] **Step 2: 在 image.rs 末尾添加 dimensions 缓存与读取函数**

在 `src/api/image.rs` 文件末尾（最后一个 `}` 之后，或在 `#[cfg(test)]` 模块之前）添加：

```rust
/// 图片尺寸缓存（moka sync）。key = 相对路径如 "2026/06/22/x.webp"。
/// 用 sync cache 而非 future cache：render_markdown_enhanced 是同步函数，不能 .await。
#[cfg(feature = "server")]
static IMAGE_DIMENSIONS_CACHE: LazyLock<SyncCache<String, (u32, u32)>> = LazyLock::new(|| {
    let ttl = std::env::var("IMAGE_DIMENSIONS_CACHE_TTL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(std::time::Duration::from_secs)
        .unwrap_or(std::time::Duration::from_secs(86400)); // 默认 24h
    SyncCache::builder().time_to_live(ttl).build()
});

/// 读取图片真实尺寸（只读 header，不解码像素）。
///
/// - `rel_path`：相对路径如 "2026/06/22/x.webp"（不含 /uploads/ 前缀和 query）
/// - 优先查缓存；miss 时读文件、解析 header、写入缓存
/// - 失败返回 None（调用方回退到不设 aspect-ratio）
#[cfg(feature = "server")]
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

/// 按扩展名分发：webp 走 zenwebp header，gif/png/jpeg 走 image crate。
#[cfg(feature = "server")]
fn read_dimensions_from_bytes(data: &[u8], path: &str) -> Option<(u32, u32)> {
    let ext = std::path::Path::new(path)
        .extension()?
        .to_str()?
        .to_lowercase();
    match ext.as_str() {
        "webp" => {
            // zenwebp 的 WebPDecoder::build 只解析 RIFF header，不解码像素
            let decoder = zenwebp::WebPDecoder::build(data).ok()?;
            let info = decoder.info();
            Some((info.width, info.height))
        }
        "gif" | "png" | "jpg" | "jpeg" => {
            // image crate 的 into_dimensions 只读 header
            let reader = image::ImageReader::new(std::io::Cursor::new(data))
                .with_guessed_format()
                .ok()?;
            reader.into_dimensions().ok()
        }
        _ => None,
    }
}
```

- [ ] **Step 3: 添加单元测试**

在 `src/api/image.rs` 的 `#[cfg(test)]` 模块内（或新建测试块）添加：

```rust
#[cfg(all(test, feature = "server"))]
mod dimensions_tests {
    use super::*;

    #[test]
    fn read_webp_dimensions_from_bytes() {
        // 构造一个 16x9 的 webp
        let img = image::DynamicImage::new_rgb8(16, 9);
        let webp_bytes = crate::webp::encode(&img, 85.0, 2).unwrap();
        let dims = read_dimensions_from_bytes(&webp_bytes, "test.webp");
        assert_eq!(dims, Some((16, 9)));
    }

    #[test]
    fn read_png_dimensions_from_bytes() {
        let img = image::DynamicImage::new_rgb8(32, 24);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let dims = read_dimensions_from_bytes(&buf.into_inner(), "test.png");
        assert_eq!(dims, Some((32, 24)));
    }

    #[test]
    fn read_dimensions_unknown_extension_returns_none() {
        let dims = read_dimensions_from_bytes(b"not an image", "test.xyz");
        assert_eq!(dims, None);
    }
}
```

- [ ] **Step 4: 运行测试验证**

Run: `cargo test dimensions_tests 2>&1 | tail -10`
Expected: 3 个测试通过。若 webp 测试失败，检查 `crate::webp::encode` 签名是否匹配（参数：img, quality, method）。

- [ ] **Step 5: Commit**

```bash
git add src/api/image.rs
git commit -m "feat(image): add image dimensions cache with header-only reading"
```

---

## Task 2: sanitizer 扩展（sanitizer.rs）

**Files:**
- Modify: `src/api/sanitizer.rs:333-361`（`clean_html` 函数）

文章正文 sanitizer 放行双层结构需要的属性。评论配置不动。

- [ ] **Step 1: 扩展 clean_html 的 extra_tag_attrs**

在 `src/api/sanitizer.rs` 的 `clean_html` 函数（约第 333 行）的 `extra_tag_attrs` vec 中：

把：
```rust
        extra_tag_attrs: vec![
            ("a", vec!["class", "aria-hidden", "aria-label"]),
            ("span", vec!["class"]),
            ("h1", vec!["id", "class"]),
            ("h2", vec!["id", "class"]),
            ("h3", vec!["id", "class"]),
            ("h4", vec!["id", "class"]),
            ("h5", vec!["id", "class"]),
            ("h6", vec!["id", "class"]),
        ],
```

改为（img 行是新增，span 加 style）：
```rust
        extra_tag_attrs: vec![
            ("a", vec!["class", "aria-hidden", "aria-label"]),
            ("img", vec!["data-src", "class", "style"]),
            ("span", vec!["class", "style"]),
            ("h1", vec!["id", "class"]),
            ("h2", vec!["id", "class"]),
            ("h3", vec!["id", "class"]),
            ("h4", vec!["id", "class"]),
            ("h5", vec!["id", "class"]),
            ("h6", vec!["id", "class"]),
        ],
```

- [ ] **Step 2: 添加测试验证新属性放行**

在 `src/api/sanitizer.rs` 的测试模块内添加：

```rust
    #[test]
    fn clean_html_allows_blur_img_attributes() {
        let input = r#"<span class="blur-img" style="--ar:16/9"><img class="blur-img-placeholder" src="/uploads/x.webp?w=20" alt="t"><img class="blur-img-full" data-src="/uploads/x.webp?w=800" alt="t"></span>"#;
        let result = clean_html(input);
        assert!(result.contains("data-src"), "data-src should be allowed");
        assert!(result.contains("blur-img-placeholder"), "class should be allowed");
        assert!(result.contains("--ar"), "style should be allowed");
    }
```

- [ ] **Step 3: 运行测试验证**

Run: `cargo test clean_html_allows_blur 2>&1 | tail -5`
Expected: 测试通过，data-src/class/style 都保留。

- [ ] **Step 4: Commit**

```bash
git add src/api/sanitizer.rs
git commit -m "feat(sanitizer): allow data-src/class/style on img for blur-up"
```

---

## Task 3: Markdown 渲染器 img 包装（markdown.rs）

**Files:**
- Modify: `src/api/markdown.rs`（`render_markdown_enhanced` 第 196-199 行 + 新增函数）

正文图转双层 wrapper。在 `push_html` 产出 HTML 后、`clean_html` 之前插入后处理。

- [ ] **Step 1: 在 render_markdown_enhanced 的返回前插入包装调用**

在 `src/api/markdown.rs` 的 `render_markdown_enhanced` 函数末尾（约第 196-199 行）：

把：
```rust
    RenderedContent {
        html: clean_html(&html),
        toc_html,
    }
```

改为：
```rust
    let html = wrap_images_with_blur(&html);
    RenderedContent {
        html: clean_html(&html),
        toc_html,
    }
```

- [ ] **Step 2: 添加 wrap_images_with_blur 函数**

在 `src/api/markdown.rs` 的 `render_markdown_enhanced` 函数之后添加：

```rust
/// 把 HTML 里的 /uploads/ 图片转成 blur-up 双层结构。
///
/// 仅处理 src 以 /uploads/ 开头的 img；外链图保持原样。
/// 对每个匹配的 img：
/// 1. 提取 src，解析出 rel_path（去 /uploads/ 前缀和 query）
/// 2. 查 get_image_dimensions 拿真实宽高，算 --ar（如 "16/9"）
/// 3. 生成 <span class="blur-img" style="--ar:.."> 包裹两层 img
#[cfg(feature = "server")]
fn wrap_images_with_blur(html: &str) -> String {
    use regex::Regex;
    use std::sync::LazyLock;

    // 匹配 pulldown-cmark 产出的 <img src="..." alt="..." /> 或 <img src="..." alt="...">
    // pulldown-cmark 格式可控：src 在前，alt 在后，属性用双引号
    static IMG_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<img\s+src="(/uploads/[^"]+)"(?:\s+alt="([^"]*)")?\s*/?>"#).unwrap()
    });

    IMG_RE.replace_all(html, |caps: &regex::Captures| {
        let src = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let alt = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        // 从 src 解析 rel_path：去 /uploads/ 前缀 + 去 query
        let rel_path = src
            .strip_prefix("/uploads/")
            .unwrap_or(src)
            .split('?')
            .next()
            .unwrap_or("");

        // 查 dimensions，算 aspect-ratio
        let ar_style = crate::api::image::get_image_dimensions(rel_path)
            .map(|(w, h)| format!(" style=\"--ar:{}:{};\"", w, h))
            .unwrap_or_default();

        // alt 转义（src/alt 来自 markdown，可能含特殊字符，但 pulldown-cmark 已转义过，这里直接用）
        let alt_attr = if alt.is_empty() {
            String::new()
        } else {
            format!(" alt=\"{}\"", alt)
        };

        format!(
            "<span class=\"blur-img\"{ar}><img class=\"blur-img-placeholder\" src=\"{src}?w=20\"{alt_attr}><img class=\"blur-img-full\" data-src=\"{src}?w=800\"{alt_attr}></span>",
            ar = ar_style,
            src = src,
            alt_attr = alt_attr,
        )
    }).to_string()
}
```

- [ ] **Step 3: 添加测试**

在 `src/api/markdown.rs` 测试模块内添加：

```rust
    #[test]
    fn wrap_images_with_blur_wraps_uploads_image() {
        // 注意：此测试依赖 uploads/ 目录下存在对应文件才能拿到 dimensions。
        // 用一个不含 dimensions 的路径验证 --ar 缺省时的结构正确性。
        let html = r#"<p><img src="/uploads/nonexistent/test.webp" alt="test"></p>"#;
        let result = wrap_images_with_blur(html);
        assert!(result.contains("blur-img-placeholder"), "should have placeholder");
        assert!(result.contains("blur-img-full"), "should have full layer");
        assert!(result.contains("?w=20"), "placeholder should use ?w=20");
        assert!(result.contains("?w=800"), "full should use ?w=800");
        assert!(result.contains("data-src"), "full should use data-src");
    }

    #[test]
    fn wrap_images_with_blur_skips_external_image() {
        let html = r#"<img src="https://example.com/img.png" alt="ext">"#;
        let result = wrap_images_with_blur(html);
        // 外链图不处理，保持原样
        assert!(!result.contains("blur-img"), "external image should not be wrapped");
    }
```

- [ ] **Step 4: 运行测试验证**

Run: `cargo test wrap_images_with_blur 2>&1 | tail -10`
Expected: 2 个测试通过。外链图保持原样，uploads 图被包装。

- [ ] **Step 5: Commit**

```bash
git add src/api/markdown.rs
git commit -m "feat(markdown): wrap uploads images with blur-up double-layer structure"
```

---

## Task 4: Blur-up CSS 样式（input.css）

**Files:**
- Modify: `input.css`（追加样式）

双层结构的视觉：占位图模糊放大、高清图淡入、aspect-ratio 预留空间。

- [ ] **Step 1: 确认 input.css 位置和暗色模式约定**

Run: `head -20 input.css`
确认 CSS 入口和暗色模式前缀（项目用 `.dark` 还是 `@media (prefers-color-scheme)`）。

- [ ] **Step 2: 在 input.css 末尾追加样式**

在 `input.css` 末尾追加：

```css
/* ========== Blur-up 渐进图片加载 ========== */
.blur-img {
  position: relative;
  display: block;
  overflow: hidden;
  aspect-ratio: var(--ar);
  /* 加载中灰底（占位图未加载完时） */
  background: var(--color-paper-code-bg, #f5f5f5);
  border-radius: 6px;
  margin: 1em 0;
}
.blur-img-placeholder {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  /* 20px 占位图放大后模糊，scale 遮边缘 */
  filter: blur(20px) saturate(1.2);
  transform: scale(1.1);
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

/* 暗色模式灰底 */
.dark .blur-img {
  background: var(--color-paper-code-bg, #2a2a2a);
}
```

- [ ] **Step 3: 构建验证 CSS 进 bundle**

Run: `make css 2>&1 | tail -3`
Expected: tailwindcss 编译成功，`public/style.css` 包含 `.blur-img`。

Run: `grep -c "blur-img-placeholder" public/style.css`
Expected: 至少 1（样式进 bundle）。

- [ ] **Step 4: Commit**

```bash
git add input.css
git commit -m "style: add blur-up progressive image loading styles"
```

---

## Task 5: ImageViewer 双层改造（image_viewer.rs）

**Files:**
- Modify: `src/components/image_viewer.rs`

卡片封面/详情封面通过 ImageViewer 渲染。改为双层结构。

- [ ] **Step 1: 添加 placeholder_params prop + dimensions 获取**

在 `src/components/image_viewer.rs` 的 `ImageViewer` 组件签名（约第 23-28 行）：

把：
```rust
#[component]
pub fn ImageViewer(
    src: String,
    #[props(default = "?w=800".to_string())] thumb_params: String,
    #[props(default = "图片".to_string())] alt: String,
    #[props(default = false)] lazy_load: bool,
) -> Element {
```

改为：
```rust
#[component]
pub fn ImageViewer(
    src: String,
    #[props(default = "?w=800".to_string())] thumb_params: String,
    #[props(default = "?w=20".to_string())] placeholder_params: String,
    #[props(default = "图片".to_string())] alt: String,
    #[props(default = false)] lazy_load: bool,
) -> Element {
```

- [ ] **Step 2: 计算 aspect-ratio（SSR 时读 dimensions）**

在组件内（`let mut is_open = use_signal(|| false);` 之后）添加：

```rust
    // 计算 aspect-ratio：SSR 时读图片真实尺寸。WASM 端不读（--ar 已在 SSR 写入 HTML）。
    // 非 /uploads/ 的外链图或读不到尺寸时不设 --ar。
    let ar_style = {
        let mut s = String::new();
        #[cfg(feature = "server")]
        {
            if let Some(rel) = src.strip_prefix("/uploads/").map(|p| p.split('?').next().unwrap_or(p)) {
                if let Some((w, h)) = crate::api::image::get_image_dimensions(rel) {
                    s = format!("--ar:{}:{};", w, h);
                }
            }
        }
        s
    };
```

- [ ] **Step 3: 改造 rsx 渲染双层结构**

把当前的缩略图 img 块（约第 80-87 行）：

```rust
        // 缩略图
        img {
            class: "cursor-pointer transition-opacity hover:opacity-90",
            src: "{thumb_src}",
            alt: "{alt}",
            loading: if lazy_load { "lazy" } else { "eager" },
            onclick: move |_| is_open.set(true),
        }
```

改为双层结构。注意：原来用 `thumb_src`（拼接了 thumb_params），现在底层用 placeholder、上层用 full。需要计算两个 URL：

在 `ar_style` 之后、`rsx!` 之前添加 URL 计算：

```rust
    // 拼接占位图 URL 和高清图 URL
    let placeholder_src = if src.contains('?') {
        format!("{}&{}", src.split('?').next().unwrap_or(&src), placeholder_params.trim_start_matches('?'))
    } else {
        format!("{}{}", src, placeholder_params)
    };
    let full_src = if src.contains('?') {
        format!("{}&{}", src.split('?').next().unwrap_or(&src), thumb_params.trim_start_matches('?'))
    } else {
        format!("{}{}", src, thumb_params)
    };
```

然后 rsx 的缩略图块改为：

```rust
        // blur-up 双层：底层占位图 + 上层高清图（data-src 由 post-content.js/前端懒加载）
        span {
            class: "blur-img",
            style: "{ar_style}",
            onclick: move |_| is_open.set(true),
            img {
                class: "blur-img-placeholder",
                src: "{placeholder_src}",
                alt: "{alt}",
                loading: if lazy_load { "lazy" } else { "eager" },
            }
            img {
                class: "blur-img-full",
                "data-src": "{full_src}",
                alt: "{alt}",
            }
        }
```

删除原来的 `let thumb_src = ...`（约第 68-77 行），它被 `placeholder_src`/`full_src` 取代。

- [ ] **Step 4: 灯箱用高清图 URL（full_src）**

灯箱的 img（约第 100-105 行）原来用 `src: "{src}"`。保持用原始 `src`（原图，无缩略图参数）作为放大图——这是点击放大看的完整图。不改灯箱。

- [ ] **Step 5: 类型检查**

Run: `dx check 2>&1 | tail -5`
Expected: No issues found。若有 `#[cfg(feature="server")]` 在 wasm32 端的编译问题，确认 `get_image_dimensions` 调用在 cfg gate 内（WASM 端 ar_style 为空字符串，双层结构仍渲染但无 --ar）。

- [ ] **Step 6: Commit**

```bash
git add src/components/image_viewer.rs
git commit -m "feat(image-viewer): render blur-up double-layer structure"
```

---

## Task 6: post-content.js 懒加载与淡入

**Files:**
- Modify: `public/js/post-content.js:50-110`（initImageZoom 函数）

当前 initImageZoom 把 img.src 改成 `?w=800`。新职责：懒加载高清图 + onload 淡入 + 灯箱改用 data-src。

- [ ] **Step 1: 重写 initImageZoom 为 blur-up 初始化**

把 `public/js/post-content.js` 的 `initImageZoom` 函数（约第 50-110 行）整体替换为：

```javascript
  function initImageZoom(root) {
    var containers = root.querySelectorAll(".blur-img");
    for (var i = 0; i < containers.length; i++) {
      var container = containers[i];
      if (container.getAttribute("data-blur-init")) continue;
      container.setAttribute("data-blur-init", "true");

      var fullImg = container.querySelector(".blur-img-full");
      if (!fullImg) continue;
      var fullSrc = fullImg.getAttribute("data-src");
      if (!fullSrc) continue;

      // 加载高清图：onload 后加 is-loaded 触发 CSS opacity 淡入
      fullImg.addEventListener("load", function () {
        this.classList.add("is-loaded");
      });

      // 懒加载：进入视口才设 src
      if ("IntersectionObserver" in window) {
        var io = new IntersectionObserver(
          function (entries) {
            entries.forEach(function (entry) {
              if (entry.isIntersecting) {
                fullImg.src = fullSrc;
                io.unobserve(container);
              }
            });
          },
          { rootMargin: "200px" }
        );
        io.observe(container);
      } else {
        // 不支持 IO：直接加载
        fullImg.src = fullSrc;
      }

      // 灯箱：点击用高清图 URL 放大
      (function (src, altText) {
        container.addEventListener("click", function (e) {
          e.preventDefault();
          var overlay = document.createElement("div");
          overlay.className = "md-image-lightbox-overlay";

          var containerEl = document.createElement("div");
          containerEl.className = "md-image-lightbox-content";

          var bigImg = document.createElement("img");
          bigImg.src = src;
          bigImg.alt = altText;

          var closeBtn = document.createElement("button");
          closeBtn.className = "md-image-lightbox-close";
          closeBtn.textContent = "\u2715";

          containerEl.appendChild(bigImg);
          containerEl.appendChild(closeBtn);
          overlay.appendChild(containerEl);
          document.body.appendChild(overlay);
          document.body.style.overflow = "hidden";

          var onKey = function (ev) {
            if (ev.key === "Escape") {
              cleanup(overlay, onKey);
            }
          };
          var cleanup = function (ol, kh) {
            closeLightbox(ol);
            document.removeEventListener("keydown", kh);
          };
          overlay.addEventListener("click", function () {
            cleanup(overlay, onKey);
          });
          containerEl.addEventListener("click", function (ev) {
            ev.stopPropagation();
          });
          closeBtn.addEventListener("click", function () {
            cleanup(overlay, onKey);
          });
          document.addEventListener("keydown", onKey);
        });
      })(fullSrc, fullImg.getAttribute("alt") || "");
    }
  }
```

- [ ] **Step 2: 验证 JS 语法**

Run: `node -c public/js/post-content.js 2>&1`
Expected: 无语法错误（`-c` 是 node 的语法检查）。

- [ ] **Step 3: Commit**

```bash
git add public/js/post-content.js
git commit -m "feat(post-content): lazy-load hi-res images with blur-up fade-in"
```

---

## Task 7: .env.example 配置

**Files:**
- Modify: `.env.example`（末尾追加）

- [ ] **Step 1: 在 .env.example 末尾追加配置**

在 `.env.example` 末尾（`IMAGE_DISK_CACHE_MAX_AGE_HOURS=168` 之后）追加：

```


# ─────────────────────────────────────────────────────────────
# 图片尺寸缓存（blur-up 占位图的 aspect-ratio 来源）
# ─────────────────────────────────────────────────────────────
# 图片尺寸缓存的 TTL，单位秒（默认 86400，即 24 小时）。
# 服务端读取图片 header 拿真实宽高，用于生成 aspect-ratio 避免布局跳动。
# 图片尺寸永不变，理论可设很长，但缓存重启会清空。
IMAGE_DIMENSIONS_CACHE_TTL_SECS=86400
```

- [ ] **Step 2: Commit**

```bash
git add .env.example
git commit -m "docs(env): document IMAGE_DIMENSIONS_CACHE_TTL_SECS"
```

---

## Task 8: 全量验证

**Files:** 无新改动，仅验证。

- [ ] **Step 1: cargo test（默认 target，含 server feature）**

Run: `cargo test 2>&1 | tail -5`
Expected: 所有测试通过（含 Task 1/2/3 新增的测试）。

- [ ] **Step 2: cargo clippy**

Run: `cargo clippy --all-targets 2>&1 | tail -5`
Expected: 无本次引入的新警告。

- [ ] **Step 3: dx check（wasm32 target，验证 ImageViewer 改造）**

Run: `dx check 2>&1 | tail -3`
Expected: No issues found。

- [ ] **Step 4: CSS 构建**

Run: `make css 2>&1 | tail -3`
Expected: 编译成功，`public/style.css` 含 `.blur-img`。

- [ ] **Step 5: JS 语法检查**

Run: `node -c public/js/post-content.js`
Expected: 无错误。

- [ ] **Step 6: 手动验证清单（需 dev server）**

启动 `make dev`，浏览器硬刷新，逐项验证：

- [ ] 文章正文图片加载时先显示模糊的占位图，无布局跳动（有 aspect-ratio 预留空间）
- [ ] 高清缩略图加载完后，opacity 平滑淡入覆盖占位图（约 0.4s 过渡）
- [ ] 滚动到视口外图片时才加载高清图（IntersectionObserver 懒加载）
- [ ] 卡片封面同样有 blur-up 效果
- [ ] 详情封面同样有 blur-up 效果
- [ ] 外链图（非 /uploads/）保持原生 img，无 blur-up
- [ ] 点击图片放大（灯箱）功能正常
- [ ] 禁用 JS 时（浏览器禁用 JS），正文图显示模糊占位图（可读但模糊）

- [ ] **Step 7: 最终 commit（如有验证中发现的小修复）**

```bash
git add -A && git commit -m "fix(blur-up): address verification findings"
```

---

## 完成标准

- 所有 Task 的 Step checkbox 打勾
- `cargo test` / `cargo clippy` / `dx check` / `make css` / `node -c` 全部通过
- 手动验证清单全部通过
- spec（`docs/superpowers/specs/2026-06-22-blur-up-images-design.md`）的 12 条验收标准全部满足

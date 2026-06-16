# Development Guide

## Performance Testing

### Prerequisites

Install hey (HTTP load generator) and samply (profiler):

```bash
brew install hey
cargo install samply
```

### Benchmark

```bash
# 1. Build release binary
make build

# 2. Start server
./target/dx/yggdrasil/release/web/server

# 3. Run load test (in another terminal)
hey -c 100 -n 100000 http://localhost:8080/
```

### Flame Graph

```bash
# 1. Build with debug symbols (required for readable flame graphs)
CARGO_PROFILE_RELEASE_DEBUG=1 make build

# Terminal 1: Start profiling
samply record -- ./target/dx/yggdrasil/release/web/server

# Terminal 2: Wait for server to start, then send load
hey -c 100 -n 100000 http://localhost:8080/

# Terminal 1: Ctrl+C after hey finishes — samply opens flame graph in browser
```

### Key Metrics to Watch

| Metric               | Description                  |
| -------------------- | ---------------------------- |
| Requests/sec         | Throughput                   |
| Average latency      | Mean response time           |
| P99 latency          | Tail latency                 |
| Status codes         | Error rate (should be 0)     |
| Latency distribution | Consistency (tight = stable) |

### Flame Graph Hotspots

| Expected Hotspot                  | Code Location    | Cause                                        |
| --------------------------------- | ---------------- | -------------------------------------------- |
| SSR rendering                     | Dioxus framework | Virtual DOM diff + render per request        |
| `deadpool` connection acquisition | `src/db/mod.rs`  | Connection pool contention under concurrency |
| `moka` cache lookup               | `src/cache.rs`   | Cache hit/miss overhead                      |
| `tokio` scheduling                | tokio runtime    | Async task dispatch                          |
| `serde` serialization             | Models           | Post/User serialization                      |

### Tuning

- `DB_POOL_SIZE` — increase if `deadpool` / `Semaphore` shows high in flame graph
- `SSR_CACHE_SECS` — increase to cache SSR output longer
- `TOKIO_WORKER_THREADS` — explicitly set worker thread count

## CI

https://git.rua.plus/api/v1/repos/xfy/yggdrasil/actions/tasks

## 代码高亮（Syntax Highlighting）

代码高亮基于 [syntect](https://docs.rs/syntect)，将代码块渲染成带 CSS class 的 HTML，配合 `public/highlight.css` 的主题规则着色。涉及四个部分：

| 文件 | 作用 |
| ---- | ---- |
| `syntaxes/*.sublime-syntax` | 各语言的语法定义（Sublime Text 格式） |
| `themes/*.tmTheme` | Catppuccin Latte（浅）/ Mocha（深）配色主题 |
| `src/highlight.rs` | 运行时高亮入口，加载语法集并渲染 HTML |
| `src/bin/generate_highlight_css.rs` | 构建期从主题生成 `public/highlight.css` |

### 渲染时机（关键）

**文章 HTML 在保存时渲染一次，固化进数据库的 `posts.content_html` 字段，读取时不再重新渲染。** `highlight_code` 只在 `render_markdown_enhanced` 内被调用，而后者只在文章创建/更新（`src/api/posts/create.rs`、`update.rs`）时触发。

这意味着：**修改语法定义后，已存在的文章不会自动刷新**，必须手动重建（见下文「刷新已有文章」）。

### 添加 / 修复某个语言的高亮

1. **编辑语法定义**：修改 `syntaxes/<Lang>.sublime-syntax`（参考同目录 `Kotlin.sublime-syntax` 的完整写法）。核心是 `expression` 上下文——它必须 `include` 所有需要识别的元素：

   ```yaml
   expression:
     - include: whitespace
     - include: comments
     - include: string-literal
     - include: declaration-keywords   # 关键字
     - include: types                   # 类型
     - include: function-declaration    # 函数声明（须在 declaration-keywords 之前）
     - include: function-calls
     - include: types-and-identifiers
   ```

   > **include 顺序很重要**：`declaration-keywords` 的裸关键字匹配会吃掉 `func name` 中的 `func`，导致后续 `function-declaration` 的多 token 匹配失败。让多 token 的规则（如 `func\s+name`）排在单 token 规则之前。

2. **验证 YAML 合法**（syntect 加载失败只会 `warn`，不会 panic，容易静默丢语法）：

   ```bash
   python3 -c "import yaml; yaml.safe_load(open('syntaxes/Swift.sublime-syntax')); print('OK')"
   ```

3. **加回归测试**：在 `src/highlight.rs` 的 `tests` 模块里加测试，断言关键字/类型/函数等产出对应的 CSS class：

   ```rust
   #[test]
   fn highlight_code_swift_keyword_and_func() {
       let result = highlight_code("func greet() {}", Some("swift"));
       assert!(result.contains("keyword"));
       assert!(result.contains("name function") || result.contains("variable function"));
   }
   ```

4. **运行测试**：

   ```bash
   cargo test --features server highlight_code_<lang> -- --nocapture
   ```

   `--nocapture` 会打印 HTML 输出，方便人眼检查每个 token 的 class 是否正确。

5. **重新生成 highlight.css**（如果新增了 scope 类型才需要，已有 scope 的颜色规则会自动覆盖）：

   ```bash
   cargo run --features server --bin generate_highlight_css
   ```

### 刷新已有文章

修改语法后，用 `/admin/posts` 页面的按钮重建文章 HTML：

- **重建内容**：仅重建 `content_html` 为空的文章
- **重建全部**：重建所有文章（含已有内容）—— 语法/渲染逻辑升级后用这个

底层调用 `rebuild_content_html(rebuild_all: bool)` server function（`src/api/posts/rebuild.rs`），单批上限 500 篇，渲染异常会被捕获汇总，不会因单篇失败中断整批。

### 调试「高亮不生效」

排查顺序（从快到慢）：

1. **先跑测试**：`cargo test --features server highlight_code_<lang> -- --nocapture`。测试直接调用 `highlight_code`，绕过 DB 和缓存，能立刻判断是语法定义问题还是运行时问题。
2. **查 DB**：测试通过但页面仍不对，查数据库里该文章的 `content_html` 是否含期望的 class——多数情况是旧 HTML 固化在 DB，需要重建：

   ```sql
   SELECT (LENGTH(content_html) - LENGTH(REPLACE(content_html, 'keyword', ''))) / LENGTH('keyword')
   FROM posts WHERE slug = '<slug>';
   ```

3. **清 SSR 缓存**：`IncrementalRenderer` 会把渲染结果持久化到 `static/` 目录（如 `static/post/<slug>/index/*.html`）。删除后重启服务器才会重新渲染。


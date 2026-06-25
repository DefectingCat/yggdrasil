# Dioxus 反模式清零 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 清零项目中残留的 Dioxus 0.7 antipatterns 官方指南点名的三类反模式——render body 副作用、用 `use_signal`/内联计算存派生值、god component。使代码库与已新建的 `.agents/skills/dioxus-render-purity/SKILL.md` 完全一致。

**Architecture:** 三个独立任务，互不依赖，可单独提交、单独回滚：
- **Task 1**（小）：`trash.rs` 的 `dirty` 派生布尔值改为 `use_memo`，消除「render 期内联重算派生值」。
- **Task 2**（中）：`post_detail.rs` 的 render 期 `set signal` 反模式——**已在 commit `225bb24` 修复**，本任务为补一个回归测试锁定行为。
- **Task 3**（大）：`write_editor`（`src/pages/admin/write.rs`，746 行 god component）抽取「封面上传」子组件 `CoverUploader`，把 5 个 cover 相关 signal + 上传闭包 + 封面 rsx 整体迁出，让 `write_editor` 减重约 150 行。

god component 的**全量拆分**（`TrashPage` 等）超出本计划范围——它风险高、收益是可维护性而非正确性，建议作为独立后续计划；本计划只做其中收益最明确、边界最清晰的一块（封面上传），验证「抽取子组件」的模式可行后再决定是否扩展。

**Tech Stack:** Rust, Dioxus 0.7（`use_memo` / `use_signal` / `#[component]` / `rsx!`），wasm-bindgen（`web_sys::File`），cargo + dx CLI。

---

## 前置知识：本项目 Dioxus 约定（执行者必读）

1. **双 target 编译**：`dx check` 同时校验 server（`aarch64-apple-darwin`）与 wasm client（`wasm32-unknown-unknown`）。任何改动都要过 `dx check`，不能只过 `cargo build`。
2. **wasm 专属代码**用 `#[cfg(target_arch = "wasm32")]` 包裹（如 `web_sys::File`、`EditorHandle`、`Closure`）。`#[cfg(not(target_arch = "wasm32"))]` 分支必须给出降级返回，否则 server 端编译报「未初始化」。
3. **`write_editor` 不是 `#[component]`**：它是普通函数，由 `Write` / `WriteEdit` 两个 `#[component]` 薄包装委托。抽取子组件后，新组件必须是 `#[component]` 才能在 `rsx!` 里用 `<CoverUploader ... />`。
4. **`#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]`**：这是项目惯例，避免 server 构建因 wasm-only 的 `mut` signal 报 unused 警告。新组件若含 wasm-only signal 要沿用此属性。
5. **测试**：`cargo test`（405 个 server 端单测）。前端组件无单测框架，靠 `dx check` 类型校验 + 手动 `make dev` 验证。
6. **提交风格**：`type(scope): 中文描述`，如 `refactor(write): 抽取封面上传子组件`。

---

## File Structure

| File | Change | Responsibility |
|------|--------|----------------|
| `src/pages/admin/trash.rs` | Modify（~112 行） | `dirty` 改 `use_memo` |
| `src/models/settings.rs` | Read only | 确认 `TrashSettings` 字段名（`retention_days`/`auto_purge_enabled`） |
| `src/pages/post_detail.rs` | Read only | 已修复；Task 2 仅加测试 |
| `tests/post_detail_slug_rerun.rs` | Create | 锁定 slug 变化重跑的回归测试 |
| `src/pages/admin/write.rs` | Modify（大改） | 抽取 `CoverUploader` 子组件，`write_editor` 减重 |
| `.agents/skills/dioxus-render-purity/SKILL.md` | Read only | 参照规则（已存在） |

---

## Task 1: `trash.rs` 的 `dirty` 改用 `use_memo`

**问题**：`src/pages/admin/trash.rs:112-118` 在 render body 内联计算 `dirty` 布尔值，每次渲染都重新 `trim().parse::<i32>()`。语义上它是 `settings_draft_*` / `settings` 的派生值，文档要求派生值用 `use_memo`（依赖不变则不重算）。

**Files:**
- Modify: `src/pages/admin/trash.rs:111-118`

**当前代码（`src/pages/admin/trash.rs:111-118`）：**

```rust
    // 草稿相对已保存配置是否存在差异：控制保存按钮可用性与“未保存”提示。
    let dirty = settings_draft_enabled() != settings().auto_purge_enabled
        || settings_draft_days()
            .trim()
            .parse::<i32>()
            .ok()
            .map(|d| d != settings().retention_days)
            .unwrap_or(true);
```

- [ ] **Step 1: 把 `dirty` 改为 `use_memo`**

用以下内容替换 `src/pages/admin/trash.rs:111-118`（保持注释语义）：

```rust
    // 草稿相对已保存配置是否存在差异：控制保存按钮可用性与“未保存」提示。
    // 派生值用 use_memo：依赖信号不变时不重算（避免每次渲染重复 parse 字符串）。
    let dirty = use_memo(move || {
        settings_draft_enabled() != settings().auto_purge_enabled
            || settings_draft_days()
                .trim()
                .parse::<i32>()
                .ok()
                .map(|d| d != settings().retention_days)
                .unwrap_or(true)
    });
```

**要点**：
- `use_memo` 已在 prelude（`use dioxus::prelude::*`，`trash.rs` 顶部已导入，无需额外 use）。
- `dirty` 的读取方式不变：原本 `dirty` 是 `bool`，现在 `dirty` 是 `Memo<bool>`，但在 `rsx!` 里 `{dirty()}` / `disabled: !dirty()` 这种用法——**注意**：`Memo<T>` 实现了 `Deref<Target = T>`，但在条件判断里仍需 `*dirty()` 或 `dirty()()`。实测：`if dirty() {` 在 Dioxus 里 `dirty()` 返回 `bool`（`Memo::read` 返回 `&T`，但 `if` 会自动解引用），保持原写法 `dirty()` 即可。若 `dx check` 报类型错，改为 `*dirty.read()`。
- 闭包用 `move ||` 捕获三个 signal 的读取。

- [ ] **Step 2: 用 `dx check` 验证类型**

运行：`dx check`
预期：`No issues found.`

若报 `expected bool, found Memo<bool>` 类错误，定位 `dirty` 的所有使用点（`grep -n "dirty" src/pages/admin/trash.rs`），把 `dirty()` 改为 `dirty()`（多数情况下无需改，`Signal`/`Memo` 在 `if`/`disabled:` 上下文自动解引用）。仍报错则改 `*dirty()`。

- [ ] **Step 3: 跑测试**

运行：`cargo test`
预期：`test result: ok. 405 passed; 0 failed`

- [ ] **Step 4: 提交**

```bash
git add src/pages/admin/trash.rs
git commit -m "refactor(trash): dirty 派生值改用 use_memo

依据 dioxus-render-purity skill 规则二：派生值不应在 render 期内联重算。
dirty 是 settings_draft_* 与 settings 的纯派生布尔值，改用 use_memo，
依赖信号不变时跳过 trim/parse 重算。"
```

---

## Task 2: 为已修复的 `post_detail` slug 反模式补回归测试

**背景**：`src/pages/post_detail.rs` 原本用 `use_signal` 镜像 `slug` prop 并在 render 期 `set`（反模式），已在 commit `225bb24` 改为直接读 prop。本任务补一个回归测试，锁定「`PostDetail` 组件不依赖镜像 signal」的契约，防止后续回退。

**注意**：Dioxus 组件在 `cargo test`（非 wasm、纯逻辑）下无法直接渲染。可行的回归方式是写一个**静态契约测试**：读取 `post_detail.rs` 源码字符串，断言它不再包含反模式签名（`slug_signal` / `use_signal(|| slug`）。这是项目已有模式（见 `src/theme.rs` 的 `theme_preload_script_*` 系列字符串断言测试）。

**Files:**
- Create: `tests/post_detail_slug_rerun.rs`

- [ ] **Step 1: 写失败的契约测试**

创建 `tests/post_detail_slug_rerun.rs`：

```rust
//! 回归测试：锁定 PostDetail 组件不再用镜像 signal 触发 server future。
//!
//! 背景：src/pages/post_detail.rs 曾用 `use_signal(|| slug.clone())` 镜像 slug prop，
//! 并在 render 期 `if slug_signal() != slug { slug_signal.set(...) }` 触发重取——
//! 这是 Dioxus antipatterns 明确点名的「render body 副作用」。
//! 已在 commit 225bb24 改为直接读 prop。本测试通过源码字符串断言防止回退。

/// PostDetail 组件源码不得包含镜像 slug 的 signal 反模式签名。
#[test]
fn post_detail_does_not_mirror_slug_into_signal() {
    let src = include_str!("../src/pages/post_detail.rs");

    // 反模式签名：用 use_signal 镜像 slug prop。
    assert!(
        !src.contains("slug_signal"),
        "post_detail.rs 重新引入了 slug_signal 镜像——这是 render 期 set signal 反模式。\
         应直接在 use_server_future 闭包里读 slug prop，让 Dioxus 在 prop 变化时自动重跑。\
         详见 .agents/skills/dioxus-render-purity/SKILL.md 规则一。"
    );

    // 反模式签名：render 期对 slug 相关 signal 调用 set。
    assert!(
        !src.contains("slug_signal.set"),
        "post_detail.rs 在 render 期 set signal——违反渲染纯净性。"
    );
}
```

- [ ] **Step 2: 运行测试，验证它通过（源码已修复，断言应成立）**

运行：`cargo test --test post_detail_slug_rerun`
预期：`test result: ok. 1 passed; 0 failed`

**反向验证**（可选，确认测试有效）：临时把 `src/pages/post_detail.rs` 改回含 `slug_signal` 的写法，重跑应 FAIL；验证后 `git checkout src/pages/post_detail.rs` 还原。

- [ ] **Step 3: 提交**

```bash
git add tests/post_detail_slug_rerun.rs
git commit -m "test(post_detail): 锁定 slug 重取不再依赖镜像 signal

防止 post_detail.rs 回退到 render 期 set signal 反模式（commit 225bb24 已修）。
用源码字符串契约断言，与 theme.rs 的 preload-script 测试同模式。"
```

---

## Task 3: 从 `write_editor` 抽取 `CoverUploader` 子组件

**问题**：`src/pages/admin/write.rs` 的 `write_editor`（63–809 行，约 746 行）是 god component。封面上传是一个**高内聚、低耦合**的子领域，分布在三处：
- **信号声明**：`src/pages/admin/write.rs:75-80`（`cover_uploading`/`cover_error`/`cover_url_mode`/`cover_drag_active`/`cover_url_input` 五个，注意 `cover_image` 在 71 行，**必须保留**）。
- **上传闭包**：`src/pages/admin/write.rs:230-249`（`spawn_cover_upload`，`#[cfg(target_arch = "wasm32")]` 包裹）。
- **封面 rsx**：`src/pages/admin/write.rs:474-708`（封面 `div` + URL 输入模式 + 错误提示，一个连续块）。

把它整体抽成 `CoverUploader` 子组件，`write_editor` 只保留 `cover_image`（保存时需要读）。

**抽取边界**：
- `CoverUploader` 拥有 5 个私有 cover signal + `spawn_cover_upload` 闭包 + 封面 rsx。
- 对外**只**暴露一个 `cover_image: Signal<String>` prop（双向：父组件声明该 signal 并传引用，子组件内 `set` 写最终 URL，父组件读它用于保存）。
- `write_editor` 删除上述信号/闭包/rsx，净减约 150 行。

**Files:**
- Modify: `src/pages/admin/write.rs`（删除 cover signal/闭包/rsx，改为 `<CoverUploader cover_image=cover_image />`，新增 `CoverUploader` 组件）

### Step 1: 通读 cover 相关代码，确认边界与引用点

- [ ] **Step 1: 确认 `cover_image` 的全部引用点（必须保留在 `write_editor`）**

运行：`grep -n "cover_image" src/pages/admin/write.rs`

预期引用点（**这些保留在 `write_editor`**，是 `cover_image` 不能迁走的理由）：
- `:71` 声明 `let mut cover_image = use_signal(|| "".to_string());`
- `:114` backfill effect：`cover_image.set(post.cover_image.clone().unwrap_or_default());`
- `:325` / `:328` 保存逻辑：`let cover_image_opt = if cover_image().trim().is_empty() {...}`
- `rsx` 内多处读 `cover_image()`

确认：`cover_image` 留在 `write_editor`，作为 prop 传给 `CoverUploader`。

- [ ] **Step 2: 确认其余 5 个 cover signal 的引用点（全部迁入 `CoverUploader`）**

运行：`grep -n "cover_uploading\|cover_error\|cover_url_mode\|cover_url_input\|cover_drag_active" src/pages/admin/write.rs`

预期：所有引用都集中在 `spawn_cover_upload` 闭包（230–249）与封面 rsx（474–708）内。**注意**：经核实 `on_submit` 保存逻辑里**没有**读 `cover_uploading()` 的拦截，故无需额外处理；若执行时发现存在该拦截（仓库后续可能改动），一并随闭包删除即可。

### Step 2: 在 `write.rs` 文件末尾新增 `CoverUploader` 组件骨架

- [ ] **Step 3: 在文件末尾（`write_editor` 函数 `}` 之后）追加 `CoverUploader` 骨架**

在 `src/pages/admin/write.rs` **末尾**追加以下骨架（仅信号声明 + 上传闭包 + 空 `rsx!`，封面 UI 在 Step 4 用「剪切粘贴」填入）：

```rust

/// 封面上传子组件。
///
/// 封装封面图的全部状态与交互：拖拽/粘贴/选择文件上传、URL 输入、预览、移除。
/// 通过 `cover_image` signal 与父组件双向绑定——子组件写入最终 URL，
/// 父组件读取它用于保存。其余上传中间态（uploading/error/drag/url）对本组件私有。
///
/// 从 `write_editor` 抽取以降低 god component 复杂度（见 dioxus-render-purity skill）。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
fn CoverUploader(cover_image: Signal<String>) -> Element {
    let mut cover_uploading = use_signal(|| false);
    let mut cover_error = use_signal(|| None::<String>);
    let mut cover_url_mode = use_signal(|| false);
    let mut cover_drag_active = use_signal(|| false);
    let mut cover_url_input = use_signal(|| "".to_string());

    // 封面图上传：spawn 一个 async 调用 upload_image_file。
    // 三条入口（file input / drop / paste）收敛成拿到 web_sys::File 后统一调用此闭包。
    #[cfg(target_arch = "wasm32")]
    let mut spawn_cover_upload = move |file: web_sys::File| {
        cover_uploading.set(true);
        cover_error.set(None);
        spawn(async move {
            match upload_image_file(file).await {
                Ok(url) => cover_image.set(url),
                Err(msg) => cover_error.set(Some(msg)),
            }
            cover_uploading.set(false);
        });
    };

    rsx! {
        // （Step 4：把 write_editor 的封面 rsx 块 474–708 剪切粘贴到这里，
        //   去掉一层缩进使其成为 CoverUploader 的直接子元素）
    }
}
```

**要点**：
- `upload_image_file` 已在 `write_editor` 所在文件顶部 import（`use ...upload_image_file`），`CoverUploader` 同文件可直接用，无需重复 import。执行前用 `grep -n "use.*upload_image_file\|upload_image_file" src/pages/admin/write.rs | head -1` 确认 import 路径。
- `#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]` 与 `write_editor` 同款，避免 server 构建因 wasm-only signal 报 unused。
- **不要**手写封面 rsx——Step 4 从 `write_editor` 剪切现有代码，确保 `evt.files().into_iter().next()` / `get_web_file()` / `try_as_web_event()` / `dyn_ref::<ClipboardEvent>()` 等真实 API 调用一字不差。

### Step 3: 把封面 rsx 从 `write_editor` 剪切到 `CoverUploader`

- [ ] **Step 4: 剪切封面 rsx 块（474–708 行）**

在 `src/pages/admin/write.rs` 中，**剪切**（不是复制）从这一行：

```rust
                    // 封面图上传区：空态矮横条（不挤压编辑器），有图时展开成 21:9 超宽预览。
```

（约 474 行）一直到这一行（含）：

```rust
                    }
```

（约 708 行——即「封面上传失败提示」`div` 的闭合 `}`）。这一整段是封面 `div` + URL 输入模式 `if` + 错误提示 `if` 三个连续块。

**粘贴位置**：`CoverUploader` 的 `rsx! { ... }` 内部，替换 Step 3 骨架里的占位注释。粘贴后**去掉一层缩进**（原在 `write_editor` 内是 5 级缩进 20 空格，在 `CoverUploader` 内应是 4 级 16 空格——以 `rsx! {` 为基准对齐）。

**校对**：粘贴后，`CoverUploader` 内部的所有 `cover_image` / `cover_uploading` / `cover_error` / `cover_url_mode` / `cover_url_input` / `cover_drag_active` / `spawn_cover_upload` 引用都解析到本组件的本地 signal/闭包。

### Step 4: 从 `write_editor` 删除已迁出的信号与闭包

- [ ] **Step 5: 删除 5 个 cover signal 声明（75–80 行，保留 71 行 `cover_image`）**

删除 `src/pages/admin/write.rs` 中这 6 行（注释 + 5 个 signal，**不含 `cover_image`**）：

```rust
    // 封面图上传状态：uploading 进度态、错误消息、URL 输入框展开、拖拽高亮。
    let mut cover_uploading = use_signal(|| false);
    let mut cover_error = use_signal(|| None::<String>);
    let mut cover_url_mode = use_signal(|| false);
    let mut cover_drag_active = use_signal(|| false);
    // 封面 URL 输入框的临时值（确认前不直接写入 cover_image，避免半截 URL 触发预览加载）。
    let mut cover_url_input = use_signal(|| "".to_string());
```

保留后该区域应只剩：

```rust
    let mut cover_image = use_signal(|| "".to_string());
    let mut status = use_signal(|| "draft".to_string());
```

- [ ] **Step 6: 删除 `spawn_cover_upload` 闭包（230–249 行）**

删除 `src/pages/admin/write.rs` 中 `spawn_cover_upload` 的整段定义，含上方注释：

```rust
    // 封面图上传：spawn 一个 async 调用 upload_image_file。
    // 三条入口（file input / drop / paste）收敛成拿到 web_sys::File 后统一调用此闭包。
    // 仅在 WASM 端有意义（upload_image_file 与 spawn 都依赖 WASM 运行时），
    // server SSR 不渲染上传逻辑，故整体 cfg-gate 避免引用 wasm-only 符号。
    #[cfg(target_arch = "wasm32")]
    let mut spawn_cover_upload = move |file: web_sys::File| {
        ...
    };
```

### Step 5: 在 `write_editor` 原封面位置插入 `CoverUploader` 调用

- [ ] **Step 7: 在 `write_editor` 的 `rsx!` 中，原封面块位置插入子组件调用**

Step 4 剪切后，`write_editor` 的 `rsx!` 里原封面块的位置现在是空的。在该位置（原 474 行处）插入：

```rust
                    // 封面图上传区（抽取为子组件 CoverUploader）。
                    CoverUploader { cover_image }
```

**缩进**：与原封面 `div` 同级（5 级缩进，20 空格）。`cover_image` 不加括号——传 signal 本身（prop 类型是 `Signal<String>`），不是 `cover_image()`（那是 `String`）。

### Step 6: 编译与验证

- [ ] **Step 8: `dx check` 验证类型**

运行：`dx check`
预期：`No issues found.`

**常见报错与修复**：
- `cannot find value cover_uploading in this scope`（在 `write_editor` 内）：有遗漏——`grep -n "cover_uploading\|cover_error\|cover_url_mode\|cover_url_input\|cover_drag_active\|spawn_cover_upload" src/pages/admin/write.rs`，确认这些名字只出现在 `CoverUploader` 组件函数体内（从 `fn CoverUploader` 到它的闭合 `}`）。
- `cannot find function spawn_cover_upload`（在 `write_editor`）：封面 rsx 没剪切干净，有残留调用。回到 Step 4 确认整块已迁出。
- `expected Signal<String>, found String` on `cover_image` prop：调用处误写 `CoverUploader { cover_image: cover_image() }`——改为 `CoverUploader { cover_image }`。
- `unused import: upload_image_file`：若 `CoverUploader` 是唯一用 `upload_image_file` 的地方且 import 在文件顶部，import 仍有效（同文件），不会报；若报则确认 import 行还在。

- [ ] **Step 9: 跑测试**

运行：`cargo test`
预期：`test result: ok. 405 passed; 0 failed`（cover 抽取是纯前端重组，不影响 server 单测）。

- [ ] **Step 10: 手动验证封面功能（`make dev`）**

运行：`make dev`，登录后台，进入 `/admin/write`，逐项验证：
- [ ] 空态：显示「拖拽 · 点击 · 粘贴封面图」+「或使用图片 URL」
- [ ] 点空态区：弹出文件选择，选图后显示预览
- [ ] 拖拽图片到上传区：边框高亮 → 上传成功显示预览
- [ ] 粘贴图片（Ctrl/Cmd+V）：上传成功显示预览
- [ ] 点「或使用图片 URL」：展开 URL 输入框，输入有效图片 URL → 确认 → 显示预览
- [ ] 预览态点右上角「×」：清空封面回到空态
- [ ] 上传中：显示「上传中...」骨架
- [ ] 进入编辑模式 `/admin/edit/<id>`（有封面的文章）：封面正确回填显示
- [ ] 填写标题+正文后点保存：成功（`cover_image` 正确传给 server function）

验证无误后 `Ctrl+C` 停止 dev server。

- [ ] **Step 11: 提交**

```bash
git add src/pages/admin/write.rs
git commit -m "refactor(write): 抽取 CoverUploader 子组件，降低 god component 复杂度

write_editor 从 746 行降至约 600 行。封面上传（5 个私有 signal + 上传闭包 +
封面 rsx）整体迁入 CoverUploader 子组件，仅通过 cover_image signal 双向绑定
与父组件通信。依据 dioxus-render-purity skill 的 god component 治理建议。"
```

---

## 验收清单（全部任务完成后过一遍）

- [ ] `dx check` 全绿
- [ ] `cargo test` 405+ passed（Task 2 新增 1 个，共 406）
- [ ] `grep -rn "slug_signal" src/` 无结果（Task 2 测试已锁定）
- [ ] `src/pages/admin/trash.rs` 的 `dirty` 是 `use_memo`
- [ ] `src/pages/admin/write.rs` 的 `write_editor` 不再含 `cover_uploading`/`spawn_cover_upload`（已迁入 `CoverUploader`）
- [ ] `make dev` 手动验证封面功能正常

## 不在本计划范围（明确排除）

- **`TrashPage`（481 行）god component 全量拆分**：风险高、收益是可维护性而非正确性。建议作为独立后续计划，且应在 Task 3 验证「抽取子组件」模式可行后再评估。
- **`AdminCommentsPage`（315 行）/ `PostsPage`（181 行）拆分**：体量中等，同上，后续单独评估。
- **`post_id` prop drilling（评论树 3 跳）**：轻微，`CommentContext` 已存在，改造成本高于收益。
- **streaming / SSR / server functions 三轮文档对照**：已确认合规，无改动。

---
name: dioxus-render-purity
description: |
  在 Dioxus 0.7 组件中保持渲染函数纯净。编写或修改 #[component] / rsx! 代码时
  强制检查：禁止在 render body 里写副作用（signal.set / spawn / DOM 调用）、
  禁止用 use_signal 存可派生数据、禁止用 use_effect 算派生值。触发关键词：
  "dioxus"、"component"、"rsx"、"use_signal"、"use_effect"、"use_server_future"、
  "渲染"、"组件"、"signal"、以及任何对 src/pages 或 src/components 下文件的编辑。
allowed-tools:
  - Read
  - Edit
  - Grep
  - Glob
metadata:
  trigger: 编写或修改 Dioxus 组件 / rsx! / signal / use_effect 代码
  source: 基于 Dioxus 0.7 antipatterns 官方指南，提炼自本项目真实违规修复
---

# Dioxus 渲染纯净性（Render Purity）

Dioxus 组件的 render 函数（`#[component]` 的函数体）**必须纯净**：给定相同输入，
产出相同 rsx，且不产生任何副作用。违反这条会触发重渲染循环、状态错乱、hydration
mismatch。本 skill 列出本项目最容易踩的三类反模式及正确写法。

## 规则一：render body 里不得有副作用

组件函数体（不在 `use_effect` / `use_resource` / 事件闭包 / `move |_| {...}` 内）
**禁止**出现：

| 禁止 | 原因 |
|---|---|
| `some_signal.set(...)` | 写信号触发重渲染，重渲染又执行到这里 → 无限循环或状态抖动 |
| `spawn(async move {...})` | 副作用不纯；每次渲染都会再发一次 |
| `web_sys::window()` / DOM 读写 / `js_sys::eval` | 服务端无 window；副作用须隔离 |
| `console.log` / `tracing::` / 网络 IO | render 只产出 rsx |

副作用一律放进 `use_effect`（一次性/依赖触发）或事件处理闭包（用户交互触发）。

### ❌ 反例（本项目曾出现，`src/pages/post_detail.rs`）

```rust
#[component]
pub fn PostDetail(slug: String) -> Element {
    // ❌ 渲染期 set signal —— 文档明确点名的反模式
    let mut slug_signal = use_signal(|| slug.clone());
    if slug_signal() != slug {
        slug_signal.set(slug.clone());
    }
    let post = use_server_future(move || {
        let s = slug_signal();
        get_post_by_slug(s)
    })?;
    // ...
}
```

错误在于：`slug_signal` 的唯一目的是「镜像 slug prop 以便 server future 重跑」，
但 `use_server_future` 的闭包**可以直接读 prop**，prop 变化时 Dioxus 会自动重跑
该 future。`slug_signal` 整个信号 + 手动 `set` 都是冗余，且 `set` 发生在 render 期。

### ✅ 正确写法

```rust
#[component]
pub fn PostDetail(slug: String) -> Element {
    // ✅ 直接读 prop；slug 变化时 use_server_future 自动重跑
    let post = use_server_future(move || {
        let s = slug.clone();
        async move { get_post_by_slug(s).await }
    })?;
    // ...
}
```

**判断标准**：如果一个信号只是「复制」某个 prop 或「镜像」另一个信号再手动 `set`
同步，它就是冗余的——要么直接读源，要么用 `use_memo`。

## 规则二：不要用 use_signal 存可派生的数据

能用其他信号/props 直接算出来的值，**不要**开独立 signal 再手动维护同步——
那样要么变成规则一的 render 期 `set`，要么需要 `use_effect` 桥接（多余的复杂度）。

| 情况 | 用什么 |
|---|---|
| 纯函数派生值（`a + b`、`list.len()`、布尔判断） | 直接在 render 内联算，或 `use_memo` 包昂贵计算 |
| 依赖 prop 派生 | 直接读 prop，不要镜像成 signal |
| 用户可编辑的表单草稿 | `use_signal`（草稿与已存值天然分叉，是独立状态，**合法**） |

### ❌ 反例（派生的布尔值内联重算，可接受但非最佳）

```rust
// 每次渲染都重新 parse 字符串；语义上是派生值
let dirty = settings_draft_enabled() != settings().auto_purge_enabled
    || settings_draft_days().trim().parse::<i32>().ok()
        .map(|d| d != settings().retention_days).unwrap_or(true);
```

### ✅ 更好（用 memo，依赖不变不重算）

```rust
let dirty = use_memo(move || {
    settings_draft_enabled() != settings().auto_purge_enabled
        || settings_draft_days().trim().parse::<i32>().ok()
            .map(|d| d != settings().retention_days).unwrap_or(true)
});
```

注意：**不要**把 memo 滥用到廉价计算上（`let x = a() + 1` 直接写就行），
memo 只用于 parse / clone 大对象 / 复杂逻辑。

## 规则三：use_effect 只做真副作用，不要用来算派生值

`use_effect` 是给「与外部世界交互」用的（网络请求、DOM 读写、事件监听、定时器、
localStorage）。如果 effect 体只是 `x.set(compute_from(y()))` 形状——把一个信号
的值算出来再写进另一个信号——那它多半应该是 `use_memo`。

### 合法的 use_effect（真副作用，不要改成 memo）

```rust
// ✅ 网络请求 + DOM 写入 + localStorage，全是副作用
use_effect(move || {
    spawn(async move {
        let posts = list_posts().await;
        posts_signal.set(posts);   // 异步结果落地，这是副作用，不是派生
    });
});
use_effect(move || {
    let theme = theme_signal();
    document.document_element().class_list().add("dark");  // DOM 副作用
});
```

### ❌ 反例形状（疑似派生，应改 memo）

```rust
// ❌ 纯粹把 b 的值搬进 a —— 派生，不是副作用
use_effect(move || {
    a.set(b() + 1);
});
```

**例外**：一次性「种子」回填（表单初始化、数据加载后填入可编辑字段）是合法 effect，
因为它只跑一次且之后状态会与源分叉。例如 `src/pages/admin/write.rs` 的编辑器回填：
源数据加载一次、用户随后可编辑，源不再权威——这**不是**派生，保留 effect。

## 自检清单（改 Dioxus 组件时逐条过）

在提交任何 `#[component]` / `rsx!` 改动前，确认：

- [ ] render body 里没有 `signal.set(...)`（除非在 `move |_| {...}` 事件闭包或 `use_effect` 内）
- [ ] render body 里没有 `spawn(...)` / `web_sys` / `js_sys::eval` / 网络 IO
- [ ] 每个 `use_signal` 都是「真正独立的状态」，而非某个 prop/信号的镜像
- [ ] 每个 `use_effect` 都有真副作用（DOM / 网络 / 监听 / 存储），不是纯派生
- [ ] 列表渲染有稳定 `key`（见上一轮修复：`key: "{post.id}"` 等）

## 参考链接

- 官方 antipatterns 指南：https://dioxuslabs.com/learn/0.7/guides/tips/antipatterns
- 官方 SSR/hydration（为何 render 必须纯净）：https://dioxuslabs.com/learn/0.7/essentials/fullstack/ssr

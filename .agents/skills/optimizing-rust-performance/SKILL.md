---
name: optimizing-rust-performance
description: |
  审查、重构或编写 Rust 代码时主动识别性能瓶颈并应用优化模式。触发关键词：
  "optimize"、"perf"、"性能优化"、"加速"、"hot path"、"热点路径"、
  "make it faster"、"reduce allocation"、"zero-copy"、"零拷贝"、
  以及任何对 .rs 文件的 review/refactor 请求、或代码中出现 Vec::remove /
  clone / Vec<String> 在循环中 / String::from / filter 后 collect 等可疑模式时。
allowed-tools:
  - Read
  - Edit
  - Grep
  - Glob
metadata:
  trigger: Rust 代码性能审查 / 重构 / 热点路径优化 / 减少 heap 分配
  source: 基于 Rust 性能优化通用最佳实践 + 真实 baseline 测试提炼
---

# Rust 性能优化（Performance Optimization）

编写或审查 Rust 代码时，**主动**识别性能瓶颈并应用优化模式。核心原则：
**按固定优先级排序**，**判断触发条件即应用**，**量化收益**，**用 profiling 验证**。

## 优化心智模型（必须按此顺序判断）

```
优化不是"想到什么改什么"。永远按此优先级评估：

1. 算法与复杂度  — O(n)→O(1)、O(n²)→O(n log n)。最大收益，优先看。
2. 内存分配     — 减少 heap 分配，栈优先，能零拷贝就零拷贝。
3. 数据布局     — cache locality、字段顺序、对齐、SoA vs AoS。
4. 并发         — 减少锁竞争，考虑 lock-free / 无锁结构。

低层级优化在高层级问题存在时收益微乎其微。先看复杂度，再看分配。
```

## 铁律：判断到触发条件就应用，不要只"提及"

> **发现反模式 → 直接改。不要说"可以考虑用 X"然后不改。**
>
> 如果触发条件命中，你必须（a）在改进后的代码里实际应用该模式，
> （b）说明改了什么、为什么快。把"应该用 Cow"挂在嘴边却不写进代码，
> 等于没优化。

## 核心模式（触发条件 → 动作）

### 模式 1：集合删除（顺序无关时）

**触发**：代码用 `.remove(index)` 删除 `Vec` 元素，且调用方不关心顺序。

**动作**：改用 `.swap_remove(index)` —— O(1)，把末尾元素换到被删位置，无内存搬移。

```rust
// ❌ O(n)：删中间元素要把后面所有元素左移
self.items.remove(index);

// ✅ O(1)：末尾元素换位，不保证顺序
self.items.swap_remove(index);
```

**收益**：O(n) → O(1) 的内存搬移。

### 模式 2：集合过滤

**触发**：代码用 `for` 循环 + 条件 `push` 到新 `Vec`，或 `filter()` 后 `collect()` 再赋值。

**动作**：用 `.retain()` / `.retain_mut()` 原地 O(n) 过滤，避免第二个 `Vec` 分配和逐元素 `clone`。

```rust
// ❌ 分配第二个 Vec + 每个元素 clone
let mut kept = Vec::new();
for item in &self.items {
    if item.in_stock { kept.push(item.clone()); }
}
self.items = kept;

// ✅ 原地过滤，零额外分配，零 clone
self.items.retain(|item| item.in_stock);
```

**收益**：省一次堆分配 + N 次 clone。

### 模式 3：所有权转移，避免 clone

**触发**：从 `&mut T` / `Option<T>` 取值时用了 `.clone()`，或想"取出旧值替换为默认"。

**动作**：
- `Option<T>` 取值 → `option.take()`（取出并留 `None`，无 clone）
- `T: Default` 取出旧值 → `std::mem::take(dest)`（旧值返回，`dest` 变默认）
- 交换值 → `std::mem::replace(dest, src)`（无深拷贝）

```rust
// ❌ clone 后原值仍在，字段没被清空（还可能是 bug）
fn clear_promo(&mut self) -> Option<PromoCode> {
    self.active_promo.clone()
}

// ✅ take：取出所有权，字段变 None，无 clone
fn clear_promo(&mut self) -> Option<PromoCode> {
    self.active_promo.take()
}
```

**收益**：消除一次堆分配（clone 的 `String`/`Vec` 等）。

### 模式 4：栈优先（短生命周期小集合）

**触发**：循环内频繁分配小而短命的 `Vec`/`String`（如"通常 2-3 个元素"的辅助返回值）。

**动作**：用 `SmallVec`/`TinyVec` 把小负载（<4 或 <8 元素）放栈上，溢出才上堆。

```rust
// ❌ 每次 tags_for 都堆分配一个 Vec（通常只有 2-3 个 tag）
fn tags_for(&self, id: u32) -> Vec<String> {
    vec![format!("tag-{id}-a"), format!("tag-{id}-b")]
}

// ✅ SmallVec：2-3 个元素常驻栈，无堆分配
fn tags_for(&self, id: u32) -> SmallVec<[String; 4]> {
    smallvec![format!("tag-{id}-a"), format!("tag-{id}-b")]
}
```

**收益**：栈分配 vs 堆分配——省掉 malloc/free 和可能的 cache miss。

### 模式 5：Copy-on-Write 延迟分配

**触发**：字符串/切片处理大多只读，偶尔才需要修改或拥有所有权；返回类型是 `String` 但其实常无需分配。

**动作**：用 `std::borrow::Cow` 封装——只读时零分配借用，真正写时才 clone。

```rust
use std::borrow::Cow;

// ❌ 永远堆分配，即使输入已经全是小写无需改动
fn normalize(name: &str) -> String {
    name.trim().to_lowercase()
}

// ✅ 只在确实需要改写（trim 砍掉字符 / 含大写）时才分配；
//    纯小写无空白的输入零分配，原样借用返回
fn normalize<'a>(name: &'a str) -> Cow<'a, str> {
    let trimmed = name.trim();
    let needs_lower = trimmed.chars().any(|c| c.is_ascii_uppercase());
    let needs_trim = trimmed.len() != name.len();
    if !needs_lower && !needs_trim {
        Cow::Borrowed(trimmed)
    } else {
        Cow::Owned(trimmed.to_lowercase())
    }
}
```

**收益**：读路径零分配；分配延迟到真正写时才发生。

## 热点路径额外模式

热点路径（每秒百万次调用的解析器/词法器/序列化）适用更激进的优化：

- **切片代替逐字符 collect**：从原 `&str`/`&[u8]` 用范围切片 `&input[start..pos]` 取词素，而不是把 `char` push 进 `Vec<char>` 再 `collect::<String>()`（省双重分配 + 双重拷贝）。
- **借用切片匹配后再拥有化**：先在借用的 `&str` 上 match 关键字（零分配），仅非关键字才 `.to_string()`。
- **ASCII 谓词代替 Unicode 谓词**：`is_ascii_digit()` / `is_ascii_alphabetic()` 代替 `is_digit(10)` / `is_alphanumeric()`（省 Unicode 表查找）——前提是你确实只需 ASCII。

## 回应规范（应用优化时必须做到）

1. **先分析瓶颈**：说明当前问题（"这会导致 O(n) 内存搬移" / "这触发一次堆分配"）。
2. **给出优化代码**：干净、生产可用的 Rust，实际应用模式（不只是提及）。
3. **量化理论收益**：说明*为什么*更快（堆 vs 栈、O(n) vs O(1)、cache locality）。
4. **提示 profiling**：提醒用 `Criterion`（微基准）或 `Flamegraph` 验证，不要盲信理论。

**量化收益示例**（写到改动说明里）：

| 改动 | 复杂度 / 分配变化 |
|---|---|
| `remove` → `swap_remove` | O(n) → O(1) 内存搬移 |
| 循环+clone → `retain` | 省 1 次堆分配 + N 次 clone |
| `clone` → `take` | 省 1 次堆分配 |
| `Vec` → `SmallVec<[T; 4]>` | 堆分配 → 栈分配（小负载时） |
| `String` → `Cow<str>` | 读路径：1 次分配 → 0 次 |

## 自检清单（提交 Rust 改动前逐条过）

- [ ] 任何 `Vec::remove` 在顺序无关处是否已换 `swap_remove`？
- [ ] 过滤操作是否用了 `retain` / `retain_mut` 而非 collect？
- [ ] 取值是否用了 `take` / `mem::take` / `mem::replace` 而非 `clone`？
- [ ] 循环内小而短命的 `Vec`/`String` 是否考虑过 `SmallVec`？
- [ ] 大多只读、偶尔写的字符串/切片是否考虑过 `Cow`？
- [ ] 热点路径是否切片取词素而非逐字符 collect？
- [ ] 每条改动是否说明了复杂度/分配收益，并提示 profiling 验证？
- [ ] 引入新依赖（`smallvec` 等）或改动公开返回类型前，确认收益配得上成本。

## 注意（应用前必读 —— 防止过度优化）

优化有成本。应用下列模式前先权衡，**别为了用模式而用**：

- **先正确再优化**：`#[inline]`、手动 SIMD、`unsafe` 等通常不是首选；先确认算法复杂度和分配已最优。
- **新增依赖有代价**：`SmallVec`/`TinyVec` 要加 crate 依赖、增加编译时间。只为"通常 2-3 个元素"就在非热点代码里引入依赖，多半不划算——优先考虑能否直接用数组 `[T; N]` / slice / 返回迭代器。
- **Cow/SmallVec 改返回类型是 API 传染**：把 `-> String` 改成 `-> Cow<'_, str>` 会把生命周期参数传染给所有调用方；把 `Token` 改成 `Token<'src>` 是全 crate 的 API 变更。改公开签名前确认收益配得上波及面；内部 `fn` 则无所谓。
- **量入为出**：Cow/SmallVec 引入复杂度，确认该路径真的是热点或高频才上。非热点的小函数直接 `Vec`/`String` 更清晰。
- **profile 验证**：理论收益不等于实测收益。用 `Criterion` 做微基准，`Flamegraph` 找真热点，别盲改。

**决策速查**：

| 情况 | 推荐 |
|---|---|
| 非热点、简单工具函数 | 直接 `Vec`/`String`/`clone`，别上模式 |
| 公开 API 返回类型 | 慎用 `Cow<'_, str>`/生命周期——会传染调用方 |
| 真热点 + 内部函数 | 放心上 Cow/SmallVec/借用切片 |
| 新增依赖 | 先看能否用 `[T; N]` / 迭代器 / 现有类型替代 |

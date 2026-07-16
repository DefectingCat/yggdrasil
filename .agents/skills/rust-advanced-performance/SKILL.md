---
name: rust-advanced-performance
description: |
  当局部代码技巧已不够、需从系统级/架构级/编译器级压榨 Rust 性能时主动应用。
  触发关键词：LTO、codegen-units、mimalloc、jemalloc、SIMD、向量化、cache locality、
  false sharing、伪共享、repr(C)、FxHash、AHash、SipHash、lock-free、无锁、AtomicU64、
  crossbeam、dashmap、zero-copy、零拷贝、Deserialize、serde 借用，以及"压榨极致性能"、
  "高并发吞吐"、"Flamegraph 找热点"、"Criterion 基准"、改 Cargo.toml profile、或线上
  服务需整体提速而非单点优化时。
allowed-tools:
  - Read
  - Edit
  - Grep
  - Glob
metadata:
  trigger: Rust 系统级性能优化 / 编译期配置 / 内存布局与缓存 / 无锁并发 / 零拷贝架构
  related: 基础 skill optimizing-rust-performance 处理局部技巧；本 skill 处理架构级手段
---

# Rust 高级性能优化（系统级 / 架构级 / 编译器级）

当局部代码技巧（见基础 skill `optimizing-rust-performance`）不足以达到目标时，
从**四个维度**寻找更大的收益：编译期配置、内存布局与缓存、并发、零拷贝架构。

**核心原则（与基础 skill 一致）**：按固定优先级评估、判断到触发条件即应用、量化收益、
**一切以 profiling 数据为准**。这些手段比局部技巧侵入性更大、成本更高，**更**需要先确认
它是真热点，别为用而用。

## 优化心智模型（系统级版，必须按此顺序判断）

```
局部技巧打不动了，再按此顺序评估系统级手段：

1. 先 profiling         — 不测量就优化是浪费。Criterion 微基准 + Flamegraph 找真热点。
                          没数据，下面三条都别动。
2. 编译期 / 配置        — 改 Cargo.toml、换分配器。不动业务代码，收益大、风险小。优先。
3. 内存布局 / 缓存      — cache locality、对齐、避免伪共享。CPU 密集型热点的核心。
4. 并发                 — 降锁竞争、原子操作、无锁结构。仅当瓶颈在多线程同步时。
5. 零拷贝               — I/O 密集型的解析/传输路径。

记住：系统级手段成本高（改全局配置 / 改数据布局 / 引入新依赖 / 改公开 API）。
局部能解决的，别上架构级。
```

## 铁律：先 profiling，再动手；判断到触发条件就应用

> **没有火焰图/基准数据，不要改 Cargo.toml、不要换分配器、不要上 SIMD。**
>
> 这些是"暴击"也是"重武器"：改 profile.release 影响所有 release 构建；换分配器影响
> 全局内存行为；SIMD/`#[repr(C)]` 改的是数据布局。**先确认目标函数/路径真的是热点**，
> 再判断下面的触发条件是否命中，命中才应用，并说明改了什么、为什么快、风险在哪。

## 一、编译期与配置黑魔法（无需改业务代码的暴击）

最高性价比：不改一行业务代码，只动 `Cargo.toml` 或入口配置。

### 模式 1：开启 LTO（Link-Time Optimization）

**触发**：release 构建追求极致体积/速度；跨 crate 调用频繁、希望编译器跨边界内联。

**动作**：在 `Cargo.toml` 开启 LTO 并限制 codegen-units。

```toml
[profile.release]
lto = true          # 跨 crate 内联 + 死代码删除
codegen-units = 1   # 单编译单元，允许更激进的全局优化（代价：编译变慢）
```

**收益**：通常 **10% ~ 20%** 运行速度提升 + 二进制体积下降。
**代价**：link 阶段显著变慢、内存占用升高。CI/release 才开，日常 dev 不开。

### 模式 2：更换内存分配器（jemalloc / mimalloc）

**触发**：多线程高频 heap 分配场景（如高并发服务、每个请求大量小对象），profiling 显示
分配/锁竞争占比高。

**动作**：用 `mimalloc` 或 `jemalloc` 替换系统默认分配器（Linux 上是 glibc malloc）。

```rust
// Cargo.toml: mimalloc = "0.1"
use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
```

**收益**：thread-local cache 降低多线程分配锁竞争，分配吞吐显著提升。
**代价**：新增依赖、稍大体积；不同工作负载提升差异大，**必须 benchmark 验证**。

## 二、内存布局与缓存友好优化（Cache Locality）

CPU 密集型热点的核心：让数据紧凑、让缓存行命中。

### 模式 3：SIMD / 自动向量化

**触发**：对超大数组做相同数学运算（矩阵、图像处理、加密、批量数值计算）。

**动作**：

- **优先靠编译器自动向量化**——写对齐友好的循环（连续切片迭代、避免循环内分支/调用）。
- 需要显式控制时用 `std::simd`（nightly）或 `wide`/`pulp` crate（stable）。

```rust
// 倾向：写编译器能自动向量化的朴素循环，而不是手撸 intrinsics
pub fn sum(xs: &[f32]) -> f32 {
    xs.iter().copied().sum::<f32>() // 连续、无分支，编译器易自动向量化
}
```

**收益**：一条指令处理多个数据，吞吐数倍提升。
**代价/注意**：手写 intrinsics/`unsafe` 易错、难维护；先确认自动向量化没命中再上显式 SIMD。

### 模式 4：调整结构体对齐与字段顺序（`#[repr(C)]` / Packing / 避免伪共享）

**触发**：

- cache locality 差：结构体字段零散、热循环里只摸其中一两个字段却把整个大结构体带进缓存。
- **伪共享（false sharing）**：多线程高频写两个变量，它们恰好落在同一缓存行（通常 64 字节），
  导致该缓存行在核心间反复失效。

**动作**：

- 字段按类型大小降序排列，提升紧凑度（Rust 默认会重排，但 `#[repr(C)]` 后顺序固定、
  需自己负责）。
- 把多线程高频写的计数器/状态用对齐填充隔开，避免同缓存行。

```rust
// ❌ 两个热变量挨着，大概率同缓存行 → false sharing
struct Counters {
    a: u64,
    b: u64,
}

// ✅ 用对齐填充强制分到不同缓存行（64 字节）
#[repr(C)]
struct Counter {
    value: u64,
    _pad: [u8; 56], // 填到 64 字节，下一个字段落到新缓存行
}
```

**收益**：减少 cache miss；消除伪共享后多线程写吞吐大幅提升。
**代价**：体积增大（padding）；`#[repr(C)]` 改变布局，与 FFI/序列化耦合时要小心。

## 三、高级数据结构与无锁并发

仅当 profiling 显示瓶颈在哈希表或线程同步时才上。

### 模式 5：用 FxHash / AHash 替代默认哈希

**触发**：`HashMap` 读写是热点，且 key 来自可信输入（内部计算、无恶意外部输入）。

**动作**：把默认 `SipHash 1-3` 换成 `fxhash` / `ahash`。

```rust
use fxhash::FxHashMap; // = HashMap<K, V, FxBuildHasher>
let mut m: FxHashMap<&str, u32> = FxHashMap::default();
```

**收益**：哈希速度快数倍，`HashMap` 读写性能暴涨。
**代价/注意**：`fxhash` 不抗 HashDoS。**面向不可信网络输入的路径绝不能换**，否则被恶意
构造 key 打成 O(n²) 拒绝服务。

### 模式 6：无锁（Lock-Free）并发

**触发**：高并发下 `Mutex` 成为瓶颈（profiling 显示锁等待 / 上下文切换占比高）。

**动作**：

- 简单计数器/状态位 → `std::sync::atomic`（`AtomicU64` / `AtomicBool`），硬件指令保证
  原子性，开销远低于锁。
- 跨线程通道 → `crossbeam-channel`，底层大量无锁（CAS）设计，吞吐远超 `std::sync::mpsc`。

```rust
use std::sync::atomic::{AtomicU64, Ordering};
static HITS: AtomicU64 = AtomicU64::new(0);
HITS.fetch_add(1, Ordering::Relaxed); // 无锁计数
```

**收益**：消除线程挂起/上下文切换，高争用场景吞吐提升明显。
**代价/注意**：无锁代码（尤其自写 CAS 循环）极难写对（ABA、memory ordering 误用）。
优先用成熟库（`crossbeam`、`dashmap`），别手撸无锁结构。

## 四、零拷贝架构（Zero-Copy）

I/O 密集型（网络/文件 → 解析）路径的杀手锏：让数据尽量不搬家。

### 模式 7：零拷贝解析（serde 借用 / `Deserialize<'a>`）

**触发**：解析 JSON/二进制时为每个字符串字段分配新 `String`，profiling 显示分配是热点。

**动作**：用 serde 的借用反序列化，让字段直接 `&'a str` 指向原始缓冲区，不分配。

```rust
use serde::Deserialize;

// ❌ 每个字段 clone 出新 String（堆分配）
#[derive(Deserialize)]
struct User {
    name: String,
    email: String,
}

// ✅ 借用：字段指向输入缓冲区，零分配（输入生命周期必须 >= 结构体）
#[derive(Deserialize)]
struct UserRef<'a> {
    name: &'a str,
    email: &'a str,
}
// let user: UserRef = serde_json::from_slice(buffer)?;
```

**收益**：读路径零分配；解析大文档时分配数从 O(n 字段) 降到 0。
**代价/注意**：借用结构体被生命周期 `'a` 绑定，输入缓冲区必须存活且不可变——
这是 API 传染（同基础 skill 的 Cow 警告），确认波及面配得上收益再改公开签名。

## 回应规范（应用系统级优化时必须做到）

1. **先给 profiling 证据**：引用火焰图/基准说明"为什么是这里"。
2. **按优先级挑手段**：先看配置（风险小），再看布局，再看并发，最后零拷贝。
3. **量化理论收益**：复杂度 / 分配 / cache 行 / 锁竞争层面的"为什么快"。
4. **说明代价与风险**：编译变慢？体积变大？不抗 HashDoS？生命周期传染？memory ordering 风险？
5. **提示验证**：改完用 Criterion 微基准对比 before/after，别盲信理论。

**量化收益参考**（写到改动说明里）：

| 改动                                   | 收益 / 代价                               |
| -------------------------------------- | ----------------------------------------- |
| `lto=true` + `codegen-units=1`         | +10%~20% 速度 / 体积下降；link 慢、内存高 |
| 系统 allocator → mimalloc/jemalloc     | 高并发分配吞吐提升；+依赖，需 benchmark   |
| 朴素循环 →（自动/显式）SIMD            | 数值批量运算吞吐数倍；显式版难维护        |
| 字段重排 / 消除 false sharing          | cache miss↓，多线程写吞吐↑；体积可能↑     |
| SipHash → FxHash/AHash                 | 哈希快数倍；**不抗 HashDoS，仅可信输入**  |
| `Mutex` → 原子 / crossbeam / dashmap   | 消除锁等待/上下文切换；自写无锁极易错     |
| `String` 字段 → `&'a str` (serde 借用) | 读路径零分配；生命周期传染 API            |

## 标准优化路径（核对清单）

面对一个要压榨极致性能的 Rust 项目，按这个顺序走：

```
1. 基础分析  ──> Criterion 写基准 + Flamegraph 抓 CPU 热点函数
       │         （没数据就别往下走）
       ▼
2. 配置先行  ──> lto=true + codegen-units=1；热点是分配就换 mimalloc/jemalloc
       │         （不动业务代码，先拿这部分收益）
       ▼
3. 代码微调  ──> 先用基础 skill：swap_remove / retain / mem::take / SmallVec / Cow
       │         （局部技巧成本最低，优先于架构级）
       ▼
4. 架构重构  ──> 内存布局/缓存（SIMD、字段重排、去伪共享）
       │         并发（FxHash、原子、crossbeam/dashmap）
       │         零拷贝（serde 借用）
```

**黄金法则：不要凭空猜测，一切以 profiling 数据为准。** 动手前先用火焰图抓出
最慢的函数，往往事半功倍。

## 自检清单（应用系统级优化前逐条过）

- [ ] 是否**先 profiling**（Criterion / Flamegraph）确认目标是真热点，再动手？
- [ ] 能否先用基础 skill 的局部技巧解决？能就别上架构级。
- [ ] LTO / codegen-units 是否只在 release profile 开，dev 保持快编译？
- [ ] 换分配器前是否 benchmark 过？不同工作负载差异大。
- [ ] SIMD 是否先试自动向量化、再考虑显式 intrinsics（后者难维护）？
- [ ] 调字段顺序/加 padding 是否评估了体积增大和 FFI/序列化耦合？
- [ ] 换 FxHash/AHash 的路径是否**只处理可信输入**，绝不被恶意 key 攻击？
- [ ] 无锁并发是否优先用成熟库（crossbeam/dashmap），而非手写 CAS？
- [ ] serde 借用改公开返回类型前，是否确认生命周期传染波及面配得上收益？
- [ ] 每条改动是否说明了收益**和代价/风险**，并提示用基准验证？

## 注意（应用前必读 —— 系统级手段更需克制）

这些手段比局部技巧重得多，应用前权衡，**别因为"听起来高级"就上**：

- **先测量再优化**：没有火焰图/基准数据，改 `Cargo.toml`、换分配器、上 SIMD 都是赌博。
  profiling 数据为准是第一原则，不是口号。
- **优先局部，其次架构**：基础 skill 的局部技巧（swap_remove / retain / Cow 等）成本最低、
  波及面最小。局部能解决就别动架构。
- **配置类副作用全局化**：`lto`/`codegen-units`/分配器影响整个 release 构建。先在分支验证，
  别直接动主干 profile。
- **安全换哈希有前提**：FxHash/AHash 不抗 HashDoS。任何面向不可信输入（HTTP body、
  外部 RPC）的 map **必须保留 SipHash**，否则引入拒绝服务漏洞。
- **无锁代码是雷区**：memory ordering / ABA 错误极难复现。用成熟库，别手写。
- **API 传染**：`&'a str` / 借用结构体的生命周期会传染所有调用方（同基础 skill 的 Cow 警告）。
  改公开签名前确认波及面配得上收益；内部 `fn` 无所谓。

**决策速查**：

| 情况                      | 推荐                                       |
| ------------------------- | ------------------------------------------ |
| 还没 profiling            | 先 Criterion + Flamegraph，别动手          |
| 局部技巧能解决            | 用基础 skill，别上架构级                   |
| release 追求极致、CI 可慢 | `lto=true` + `codegen-units=1`             |
| 多线程高频分配是热点      | benchmark 后换 mimalloc/jemalloc           |
| CPU 密集 + 批量数值运算   | 先试自动向量化，不够再显式 SIMD            |
| 多线程高频写相邻变量      | 对齐填充消除 false sharing                 |
| 可信输入 + HashMap 是热点 | FxHash/AHash；**不可信输入保留 SipHash**   |
| 高争用锁是瓶颈            | 原子计数；通道用 crossbeam；map 用 dashmap |
| I/O 解析分配是热点        | serde 借用 `&'a str`（注意生命周期传染）   |

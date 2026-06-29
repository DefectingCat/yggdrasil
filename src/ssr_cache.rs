//! SSR 增量渲染缓存失效的未来就绪基础设施。
//!
//! 本模块维护一个全局单调递增的世代号（generation）。文章写入成功后调用方会
//! 使其递增，从而**标记** SSR 渲染结果已过期。然而：
//!
//! **Dioxus 0.7 的增量渲染器使用请求 URI 的 `path_and_query()` 作为内部缓存键，
//! 且没有暴露公开 API 供外部代码自定义缓存键或按路由失效已渲染页面。**
//! 因此，当前世代号并**不会**实际使 Dioxus 的 SSR 缓存失效；它只是为未来 API
//! 准备好状态，并在请求/响应中提供可观测性。
//!
//! 在 Dioxus 提供以下任一能力之前，有效的 SSR 缓存失效手段仍是调低
//! `SSR_CACHE_SECS` 这一兜底 TTL：
//! - 自定义增量渲染缓存键的回调；或
//! - 从 server function 内部按路由失效缓存的公开 API。
//!
//! 当前实现：
//! - `bump_global_generation()` / `current_global_generation()`：全局世代号。
//! - `SsrGeneration`：注入到请求扩展中的类型；未来 Dioxus 支持读取扩展生成
//!   缓存键时可直接使用。
//! - `src/main.rs` 的中间件把当前世代号附加到 `X-SSR-Generation` 响应头
//!   （仅 GET 请求），便于调试与监控。
//!
//! 仅在启用 `server` feature 时编译。

#![cfg(feature = "server")]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;

/// 全局 SSR 世代号。
///
/// 任何文章写入操作都会使其递增，从而让所有基于该全局世代的 SSR 缓存键在未来
/// Dioxus 支持自定义缓存键时失效。
static GLOBAL_GENERATION: LazyLock<AtomicU64> = LazyLock::new(AtomicU64::default);

/// 注入到请求扩展中的当前 SSR 世代号。
///
/// 这是为未来 Dioxus 支持自定义 SSR 缓存键预留的钩子。当前 Dioxus 0.7 的渲染器
/// 不会读取此扩展。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SsrGeneration(pub u64);

/// 原子递增并返回新的全局世代号。
pub fn bump_global_generation() -> u64 {
    GLOBAL_GENERATION
        .fetch_add(1, Ordering::SeqCst)
        .wrapping_add(1)
}

/// 返回当前全局世代号。
pub fn current_global_generation() -> u64 {
    GLOBAL_GENERATION.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn global_generation_is_monotonic() {
        let before = current_global_generation();
        let g1 = bump_global_generation();
        let g2 = bump_global_generation();
        let current = current_global_generation();

        assert!(g1 > before || g1 == 1);
        assert!(g2 > g1);
        assert_eq!(current, g2);
    }
}

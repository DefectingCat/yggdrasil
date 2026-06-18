//! 数据库连接获取的指数退避重试策略。
//!
//! 取代 pool.rs 中固定 2s 间隔的重试：每次重试间隔 = base * 2^attempt，
//! 再叠加 [0, base) 的随机 jitter，避免多请求同步重试形成惊群。
//! 仅在 `feature = "server"` 时编译。

#[cfg(feature = "server")]
use std::time::Duration;

/// 退避基准间隔（首次重试前的等待约为 base，随后翻倍）。
#[cfg(feature = "server")]
const BASE_BACKOFF: Duration = Duration::from_millis(200);

/// 最大重试次数（不含首次尝试）。
#[cfg(feature = "server")]
pub const MAX_RETRIES: u32 = 3;

/// 计算第 `attempt` 次重试（attempt 从 0 开始）前的等待时长。
///
/// 公式：base * 2^attempt，再叠加 [0, base) 的 jitter。
/// jitter 由调用方传入的随机比例 [0.0, 1.0) 决定，便于测试时锁定为 0。
#[cfg(feature = "server")]
pub fn backoff_for(attempt: u32, jitter_ratio: f64) -> Duration {
    debug_assert!((0.0..=1.0).contains(&jitter_ratio));
    let exp = u32::checked_shl(1, attempt).unwrap_or(1 << 30);
    let base_ms = BASE_BACKOFF.as_millis() as u64;
    let core = base_ms.saturating_mul(exp as u64);
    let jitter = (base_ms as f64 * jitter_ratio) as u64;
    Duration::from_millis(core.saturating_add(jitter))
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_exponentially_without_jitter() {
        // jitter=0 时序列应严格翻倍：200, 400, 800 ms。
        assert_eq!(backoff_for(0, 0.0), Duration::from_millis(200));
        assert_eq!(backoff_for(1, 0.0), Duration::from_millis(400));
        assert_eq!(backoff_for(2, 0.0), Duration::from_millis(800));
    }

    #[test]
    fn backoff_includes_jitter_within_base_range() {
        // jitter_ratio=0.5 时在 core 上叠加 base*0.5 = 100ms。
        assert_eq!(backoff_for(0, 0.5), Duration::from_millis(300));
        assert_eq!(backoff_for(1, 0.5), Duration::from_millis(500));
    }

    #[test]
    fn backoff_clamps_large_attempt() {
        // 超大 attempt 不应 panic，应靠 saturating 保护返回一个大但有界的值。
        let d = backoff_for(40, 0.0);
        assert!(d.as_millis() > 0);
    }
}

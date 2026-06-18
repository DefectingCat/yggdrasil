//! PostgreSQL 连接池实现。
//!
//! 仅在启用 `server` feature 时编译，使用 deadpool-postgres 管理连接池，
//! 并通过 `std::sync::LazyLock` 在首次访问时延迟初始化全局连接池。
//! `get_conn` 失败时按指数退避 + jitter 重试（见 `retry` 模块），以应对瞬时连接失败。

use std::sync::LazyLock;
use std::time::Duration;

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;

/// 全局数据库连接池，基于 `DATABASE_URL` 环境变量延迟初始化。
///
/// 最大连接数可通过 `DB_POOL_SIZE` 环境变量调整，默认 20。
pub static DB_POOL: LazyLock<Pool> = LazyLock::new(|| {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable not set");
    let mut pg_cfg = db_url
        .parse::<tokio_postgres::Config>()
        .expect("Invalid DATABASE_URL format");

    // statement_timeout：防止单条慢查询（如全表扫搜索）长时间占用连接拖垮池。
    // 默认 30s，可由 STATEMENT_TIMEOUT_SECS 覆盖（L6）。
    let statement_timeout_secs = std::env::var("STATEMENT_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(30);
    // 通过 libpq options 传递 GUC；tokio-postgres 在建连时执行。
    pg_cfg.options(format!("-c statement_timeout={}", statement_timeout_secs * 1000));

    // 使用 Fast 回收策略：归还连接时不额外发 SELECT 1 验证，直接复用。
    // Verified 在高并发下会为每次 get() 增加一次往返；Fast 依赖 tokio-postgres
    // 在使用时自然报错，由 get_conn 的重试层兜底。
    let mgr_cfg = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };
    let mgr = Manager::from_config(pg_cfg, NoTls, mgr_cfg);

    Pool::builder(mgr)
        .max_size(
            std::env::var("DB_POOL_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
        )
        .wait_timeout(Some(Duration::from_secs(10)))
        .create_timeout(Some(Duration::from_secs(10)))
        .recycle_timeout(Some(Duration::from_secs(5)))
        .runtime(Runtime::Tokio1)
        .build()
        .expect("Failed to create database connection pool")
});

/// 从全局连接池获取一个数据库连接，失败时按指数退避 + jitter 重试。
///
/// 退避策略见 `retry::backoff_for`。仅对 Backend/Postgres 错误（DB 不可达）重试；
/// Timeout（池满）直接返回，让上层限流兜底，避免雪崩（L6）。
/// 若所有重试均失败，返回最后一次的 PoolError。
pub async fn get_conn() -> Result<deadpool_postgres::Object, deadpool_postgres::PoolError> {
    use rand::Rng;

    let mut last_err = None;
    for attempt in 0..=crate::db::retry::MAX_RETRIES {
        match DB_POOL.get().await {
            Ok(conn) => return Ok(conn),
            Err(e) => {
                // Timeout（池满）不重试：快速失败让上层限流兜底，避免雪崩。
                // Backend/Postgres（DB 不可达）才退避重试。
                let is_timeout = matches!(e, deadpool_postgres::PoolError::Timeout(_));
                last_err = Some(e);
                if !is_timeout && attempt < crate::db::retry::MAX_RETRIES {
                    let jitter = rand::thread_rng().gen::<f64>();
                    let delay = crate::db::retry::backoff_for(attempt, jitter);
                    tracing::warn!(
                        "DB connection attempt {} failed (backend error), retrying in {:?}: {:?}",
                        attempt + 1,
                        delay,
                        last_err.as_ref().unwrap(),
                    );
                    tokio::time::sleep(delay).await;
                } else if is_timeout {
                    // 池满：立即返回，不再 sleep。
                    break;
                }
            }
        }
    }
    Err(last_err.unwrap())
}

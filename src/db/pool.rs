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
    let pg_cfg = db_url
        .parse::<tokio_postgres::Config>()
        .expect("Invalid DATABASE_URL format");

    // 使用 Verified 回收策略，确保归还的连接仍然可用，避免 DB 重启后拿到死连接。
    let mgr_cfg = ManagerConfig {
        recycling_method: RecyclingMethod::Verified,
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
/// 退避策略见 `retry::backoff_for`。jitter 使用 `rand` 生成 [0,1) 随机数，
/// 避免多请求同步重试形成惊群。若所有尝试均失败，返回最后一次的 PoolError。
pub async fn get_conn() -> Result<deadpool_postgres::Object, deadpool_postgres::PoolError> {
    use rand::Rng;

    let mut last_err = None;
    for attempt in 0..=crate::db::retry::MAX_RETRIES {
        match DB_POOL.get().await {
            Ok(conn) => return Ok(conn),
            Err(e) => {
                last_err = Some(e);
                if attempt < crate::db::retry::MAX_RETRIES {
                    let jitter = rand::thread_rng().gen::<f64>();
                    let delay = crate::db::retry::backoff_for(attempt, jitter);
                    tracing::warn!(
                        "DB connection attempt {} failed, retrying in {:?}: {:?}",
                        attempt + 1,
                        delay,
                        last_err.as_ref().unwrap(),
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    Err(last_err.unwrap())
}

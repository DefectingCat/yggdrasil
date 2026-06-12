//! PostgreSQL 连接池实现。
//!
//! 仅在启用 `server` feature 时编译，使用 deadpool-postgres 管理连接池，
//! 并通过 `std::sync::LazyLock` 在首次访问时延迟初始化全局连接池。
//! `get_conn` 失败时按固定 2 秒间隔进行简单重试，以应对瞬时连接失败。

use std::sync::LazyLock;
use std::time::Duration;

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::NoTls;

/// 全局数据库连接池，基于 `DATABASE_URL` 环境变量延迟初始化。
///
/// 最大连接数可通过 `DB_POOL_SIZE` 环境变量调整，默认 20。
pub static DB_POOL: LazyLock<Pool> = LazyLock::new(|| {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable not set");
    let pg_cfg = db_url
        .parse::<tokio_postgres::Config>()
        .expect("Invalid DATABASE_URL format");

    // 使用 Fast 回收策略，避免每次归还连接时执行额外查询。
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
        .build()
        .expect("Failed to create database connection pool")
});

/// 最大重试次数。
const MAX_RETRIES: u32 = 3;

/// 每次重试之间的固定等待时间。
const RETRY_DELAY: Duration = Duration::from_secs(2);

/// 从全局连接池获取一个数据库连接，失败时按 `MAX_RETRIES` 进行重试。
///
/// 若所有尝试均失败，则返回最后一次遇到的 PoolError。
pub async fn get_conn() -> Result<deadpool_postgres::Object, deadpool_postgres::PoolError> {
    let mut last_err = None;
    for attempt in 0..=MAX_RETRIES {
        match DB_POOL.get().await {
            Ok(conn) => return Ok(conn),
            Err(e) => {
                if attempt < MAX_RETRIES {
                    tracing::warn!(
                        "DB connection attempt {} failed, retrying in {:?}: {:?}",
                        attempt + 1,
                        RETRY_DELAY,
                        e
                    );
                    tokio::time::sleep(RETRY_DELAY).await;
                }
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap())
}

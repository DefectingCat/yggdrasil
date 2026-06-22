//! PostgreSQL 连接池实现。
//!
//! 仅在启用 `server` feature 时编译，使用 deadpool-postgres 管理连接池，
//! 并通过 `std::sync::LazyLock` 在首次访问时延迟初始化全局连接池。
//! `get_conn` 失败时按指数退避 + jitter 重试（见 `retry` 模块），以应对瞬时连接失败。
//!
//! 启动期的重试窗口更长（见 `get_conn_for_startup`），并配合 `main.rs` 的前置校验
//!（`validate_database_url`）让所有启动期致命错误走统一友好的 `exit(1)` 路径。

use std::sync::LazyLock;
use std::time::Duration;

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;

/// 解析 `DATABASE_URL` 并注入 `statement_timeout`，返回配置好的 `tokio_postgres::Config`。
///
/// 把原本写死在 `DB_POOL` LazyLock 闭包里的逻辑抽出来，便于：
/// - `main.rs` 启动早期做前置校验（`validate_database_url`）；
/// - LazyLock 闭包退化为不可达的防御性代码。
///
/// 返回 `Err(String)` 而非 panic，调用方决定如何向用户报告错误。
fn build_pg_config() -> Result<tokio_postgres::Config, String> {
    let db_url = std::env::var("DATABASE_URL").map_err(|_| {
        "DATABASE_URL environment variable not set".to_string()
    })?;
    let mut pg_cfg = db_url
        .parse::<tokio_postgres::Config>()
        .map_err(|e| format!("Invalid DATABASE_URL format: {e}"))?;

    // statement_timeout：防止单条慢查询（如全表扫搜索）长时间占用连接拖垮池。
    // 默认 30s，可由 STATEMENT_TIMEOUT_SECS 覆盖。
    let statement_timeout_secs = std::env::var("STATEMENT_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(30);
    // 通过 libpq options 传递 GUC；tokio-postgres 在建连时执行。
    pg_cfg.options(format!("-c statement_timeout={}", statement_timeout_secs * 1000));

    Ok(pg_cfg)
}

/// 启动早期校验：`DATABASE_URL` 格式合法 + `DB_POOL_SIZE` 为正数。
///
/// 供 `main.rs` 在 `DB_POOL` LazyLock 被触碰之前调用，让 URL 格式错误、池大小非法
/// 这类用户可修复的配置问题走统一友好的 `tracing::error!` + `exit(1)` 路径，
/// 而不是触发 LazyLock 闭包里的 `.expect()` panic。
///
/// 返回 `Err(String)` 时，字符串已是面向用户的错误描述。
pub fn validate_database_url() -> Result<(), String> {
    build_pg_config()?;

    // 同步校验池大小，避免 LazyLock 闭包里 `unwrap_or(20)` 静默吞掉非法值。
    if let Ok(s) = std::env::var("DB_POOL_SIZE") {
        match s.parse::<usize>() {
            Ok(n) if n > 0 => {}
            Ok(_) => return Err("DB_POOL_SIZE is not a positive integer".to_string()),
            Err(e) => return Err(format!("Invalid DB_POOL_SIZE value: {e}")),
        }
    }
    Ok(())
}

/// 全局数据库连接池。
///
/// **不可达的防御性 panic**：`main.rs` 启动时已通过 `validate_database_url()` 前置校验
/// `DATABASE_URL` 格式与 `DB_POOL_SIZE`，因此在真实运行路径上本闭包里的 `.expect()`
/// 永远不会触发。保留 `.expect()` 只是为了满足 `LazyLock` 必须返回 `T`（而非 `Result`）
/// 的类型约束——若这里真的 panic，说明 `validate_database_url` 与本闭包逻辑不一致，
/// 属于代码 bug 而非用户错误。
pub static DB_POOL: LazyLock<Pool> = LazyLock::new(|| {
    // 前置校验已保证配置合法；闭包里直接 expect 以满足 LazyLock 的类型约束。
    let pg_cfg = build_pg_config()
        .expect("DATABASE_URL should have been validated at startup; validate_database_url() was not called");

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
/// 这是**运行期**获取连接的路径：反雪崩导向——快速失败（约 1.6s 后放弃），
/// 让上层限流兜底。**启动期**（迁移）请用 [`get_conn_for_startup`]，它有一个
/// 更长的可配置重试窗口，专为“DB 还没起来”的场景设计。
///
/// 退避策略见 `retry::backoff_for`。仅对 Backend/Postgres 错误（DB 不可达）重试；
/// Timeout（池满）直接返回，让上层限流兜底，避免雪崩。
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

/// 启动期专用：在可配置的时间窗口内反复尝试连接数据库，专治“DB 还没起来”。
///
/// 与运行期的 [`get_conn`] 区别：
/// - **没有反雪崩约束**：启动时只有这一个进程在连，不会雪崩，可以放心长重试。
/// - **固定间隔重试**（而非指数退避）：启动场景下 DB 要么起来要么没起来，
///   固定 500ms 轮询比指数退避更可预测，也更贴合 `pg_isready` 式的等待语义。
/// - **以总时长为终止条件**（而非次数）：对运维更直观——"给 DB 30 秒起来"。
///
/// 超时窗口由 `MIGRATE_STARTUP_TIMEOUT_SECS` 控制，默认 30 秒。窗口用尽后返回
/// 最后一次错误，由调用方（`main.rs`）决定如何向用户报告。
///
/// 适用 docker-compose（无 healthcheck）、本机忘启 Postgres 等“DB 起得比 app 慢”的场景。
pub async fn get_conn_for_startup(
) -> Result<deadpool_postgres::Object, deadpool_postgres::PoolError> {
    let timeout_secs = std::env::var("MIGRATE_STARTUP_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let retry_interval = Duration::from_millis(500);

    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match DB_POOL.get().await {
            Ok(conn) => {
                if attempt > 1 {
                    tracing::info!(
                        "connected to database after {} attempt(s)",
                        attempt
                    );
                }
                return Ok(conn);
            }
            Err(e) => {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    return Err(e);
                }
                tracing::warn!(
                    "startup DB connection attempt {} failed, ~{}s remaining until giving up: {:?}",
                    attempt,
                    remaining.as_secs(),
                    e,
                );
                // 不要睡过 deadline，避免超出用户配置的窗口。
                let sleep = std::cmp::min(retry_interval, remaining);
                tokio::time::sleep(sleep).await;
            }
        }
    }
}

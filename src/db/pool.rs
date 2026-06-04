use std::sync::LazyLock;
use std::time::Duration;

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::NoTls;

pub static DB_POOL: LazyLock<Pool> = LazyLock::new(|| {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable not set");
    let pg_cfg = db_url
        .parse::<tokio_postgres::Config>()
        .expect("Invalid DATABASE_URL format");

    let mgr_cfg = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };
    let mgr = Manager::from_config(pg_cfg, NoTls, mgr_cfg);

    Pool::builder(mgr)
        .max_size(10)
        .build()
        .expect("Failed to create database connection pool")
});

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(2);

pub async fn get_conn() -> Result<deadpool_postgres::Object, deadpool_postgres::PoolError> {
    let mut last_err = None;
    for attempt in 0..=MAX_RETRIES {
        match DB_POOL.get().await {
            Ok(conn) => return Ok(conn),
            Err(e) => {
                if attempt < MAX_RETRIES {
                    tracing::warn!("DB connection attempt {} failed, retrying in {:?}: {:?}", attempt + 1, RETRY_DELAY, e);
                    tokio::time::sleep(RETRY_DELAY).await;
                }
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap())
}

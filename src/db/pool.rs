use std::sync::LazyLock;

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

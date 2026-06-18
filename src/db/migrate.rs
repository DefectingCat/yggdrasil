//! 数据库迁移运行器。
//!
//! 在服务器启动时（`dioxus::server::serve()` 之前）自动执行迁移。
//! 设计要点：
//! - 迁移 SQL 通过 `include_str!` 内联进二进制，部署只需单个二进制。
//! - `schema_migrations` 表记录已应用版本，避免重复执行。
//! - 每个迁移在独立事务里执行，失败自动回滚，版本行不会写入。
//! - 咨询锁（`pg_advisory_lock`）保证多实例启动时只有一个进程执行迁移。
//!
//! 仅在 `feature = "server"` 时编译。

/// 咨询锁的固定 key。Postgres 咨询锁是数据库级唯一的；
/// 这里用一个项目专属的大整数，避免与同库其它应用冲突。
/// 该值无语义，仅要求唯一性。
#[allow(dead_code)] // used by run() in Task 3
const ADVISORY_LOCK_KEY: i64 = 0x5947_4752_4153_494C;

/// 所有迁移的 (version, sql) 列表，按 version 升序排列。
///
/// 新增迁移时：
/// 1. 在 `migrations/` 下创建 `NNN_描述.sql`
/// 2. 在本数组末尾追加一行 `("NNN", include_str!("../../migrations/NNN_描述.sql"))`
///
/// version 用字符串而非整数，与文件名前缀直接对应，且未来可支持日期戳/语义版本。
const MIGRATIONS: &[(&str, &str)] = &[
    ("001", include_str!("../../migrations/001_init.sql")),
    ("002", include_str!("../../migrations/002_posts.sql")),
    ("003", include_str!("../../migrations/003_indexes.sql")),
    ("004", include_str!("../../migrations/004_search_trgm.sql")),
    ("005", include_str!("../../migrations/005_comments.sql")),
    ("006", include_str!("../../migrations/006_add_toc_html.sql")),
    ("007", include_str!("../../migrations/007_settings.sql")),
    ("008", include_str!("../../migrations/008_comments_cascade.sql")),
    (
        "009",
        include_str!("../../migrations/009_cleanup_duplicate_indexes.sql"),
    ),
    (
        "010",
        include_str!("../../migrations/010_post_word_counts.sql"),
    ),
    ("011", include_str!("../../migrations/011_perf_indexes.sql")),
    (
        "012",
        include_str!("../../migrations/012_session_generation.sql"),
    ),
    (
        "013",
        include_str!("../../migrations/013_comment_content_hash_index.sql"),
    ),
    (
        "014",
        include_str!("../../migrations/014_drop_ineffective_trgm_index.sql"),
    ),
    // 新增迁移在此追加，同时在 migrations/ 下创建对应 .sql 文件。
];

/// 迁移执行错误。
#[derive(Debug)]
pub enum MigrateError {
    /// 无法从连接池获取连接。
    Pool(deadpool_postgres::PoolError),
    /// 执行查询（建表、查版本、咨询锁等）失败。
    Query(tokio_postgres::Error),
    /// 某个具体迁移执行失败（包含版本号便于定位）。
    Apply {
        version: String,
        source: tokio_postgres::Error,
    },
}

impl From<deadpool_postgres::PoolError> for MigrateError {
    fn from(e: deadpool_postgres::PoolError) -> Self {
        MigrateError::Pool(e)
    }
}

impl From<tokio_postgres::Error> for MigrateError {
    fn from(e: tokio_postgres::Error) -> Self {
        MigrateError::Query(e)
    }
}

impl std::fmt::Display for MigrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrateError::Pool(e) => write!(f, "database pool error: {}", e),
            MigrateError::Query(e) => write!(f, "database query error: {}", e),
            MigrateError::Apply { version, source } => {
                write!(f, "migration {} failed: {}", version, source)
            }
        }
    }
}

impl std::error::Error for MigrateError {}

/// 执行所有未应用的迁移。
///
/// 在 `main.rs` 启动时调用一次。流程：
/// 1. 获取一个独占连接（咨询锁是 session 级，需在同一连接上 lock/unlock）
/// 2. 抢咨询锁（多实例启动时串行化）
/// 3. 确保 `schema_migrations` 表存在
/// 4. 查询已应用版本集合
/// 5. 按序应用未应用的迁移（每个一个事务）
/// 6. 释放咨询锁
///
/// 失败时返回错误；调用方（`main.rs`）应让进程退出，避免启动半残服务。
pub async fn run() -> Result<(), MigrateError> {
    unimplemented!("filled in by Task 3")
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_sorted_ascending() {
        let mut sorted = MIGRATIONS.iter().map(|(v, _)| *v).collect::<Vec<_>>();
        sorted.sort_unstable();
        let original: Vec<&str> = MIGRATIONS.iter().map(|(v, _)| *v).collect();
        assert_eq!(original, sorted, "MIGRATIONS must be in ascending version order");
    }

    #[test]
    fn migrations_have_unique_versions() {
        let mut versions: Vec<&str> = MIGRATIONS.iter().map(|(v, _)| *v).collect();
        let total = versions.len();
        versions.sort_unstable();
        versions.dedup();
        assert_eq!(versions.len(), total, "MIGRATIONS has duplicate version strings");
    }

    #[test]
    fn migrations_non_empty() {
        assert!(!MIGRATIONS.is_empty(), "MIGRATIONS must not be empty");
    }
}

//! 服务端共享工具（仅 `feature = "server"` 编译）。
//!
//! 集中跨模块重复的服务端常量与工具函数（issue #7 重复常量去重）。

#![cfg(feature = "server")]

use sha2::{Digest, Sha256};

/// 明文 token / 任意字符串 → SHA-256 hex。
///
/// 此前 `auth/session.rs` 与 `mcp/auth.rs` 各有一份逐字相同的实现。
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// 邮箱格式正则（`^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$`）。
///
/// 此前 `api/auth.rs` 与 `api/comments/helpers.rs` 各有一份逐字相同的 LazyLock。
pub static EMAIL_REGEX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .expect("EMAIL_REGEX 正则模式应在编译期通过校验")
});

/// 上传文件大小上限（5 MiB）。
///
/// 此前 `api/upload.rs` 与 `mcp/tools/media.rs` 各定义一份相同的常量。
pub const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;

/// 启动期数据库迁移超时窗口（秒），由 `MIGRATE_STARTUP_TIMEOUT_SECS` 控制，默认 30。
///
/// 此前 `main.rs` 与 `db/pool.rs`（`get_conn_for_startup`、`ensure_database_exists` 两处）
/// 各有一份逐字相同的 `.ok().and_then(parse).unwrap_or(30)` 解析链。
pub fn parse_migrate_startup_timeout() -> u64 {
    std::env::var("MIGRATE_STARTUP_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30)
}

//! 数据库连接模块。
//!
//! 本模块根据 `server` feature 的启用情况提供两套实现：
//! - 启用 `server` 时，从 `pool` 子模块暴露真实的 PostgreSQL 连接池；
//! - 未启用 `server` 时（例如仅编译 WASM 前端），提供一个 `DummyPool` stub，
//!   使代码在缺少数据库依赖的情况下仍能编译通过。
//!
//! 这种 stub 模式是 Dioxus fullstack 项目的常见做法：服务端函数体在 WASM 构建时会被剥离，
//! 但模块结构必须保持一致，因此需要一个占位实现来满足编译器的符号解析。

/// 错误格式化工具：把 `std::error::Error` 的 source 链完整展开为字符串。
///
/// 存在的原因：`tokio_postgres::Error` 的 `Display` 对 DB 侧错误只会打印
/// 无信息量的占位串 `db error`，真正的消息文本（如
/// `column "x" of relation "y" already exists`、SQLSTATE、约束名）藏在
/// `source()` 链里的 `postgres::error::DbError`。不主动遍历链，日志和错误
/// 字符串就会全部折叠成 `db error`，无法定位失败原因。
///
/// 用法：`format!("...: {}", format_with_sources(&e))` 或直接
/// `format_with_sources(&e)` 得到完整的 `e: cause: deeper cause`。
#[cfg(feature = "server")]
pub fn format_with_sources(e: &dyn std::error::Error) -> String {
    use std::fmt::Write;
    let mut s = e.to_string();
    let mut cur: &dyn std::error::Error = e;
    while let Some(next) = cur.source() {
        // 跳过与外层 Display 完全相同的占位层（如 tokio_postgres 的 `db error`），
        // 避免输出 `db error: db error` 这种重复。只在能带来新信息时追加。
        let next_disp = next.to_string();
        if !next_disp.is_empty() && next_disp != s {
            let _ = write!(s, ": {next_disp}");
        }
        cur = next;
    }
    s
}

/// 真实的 PostgreSQL 连接池实现，仅在启用 server feature 时编译。
#[cfg(feature = "server")]
pub mod pool;

/// 连接获取的指数退避重试策略，仅在启用 server feature 时编译。
#[cfg(feature = "server")]
pub mod retry;

/// 数据库迁移运行器，仅在启用 server feature 时编译。
#[cfg(feature = "server")]
pub mod migrate;

/// 占位连接池实现，仅在不启用 server feature 时编译。
///
/// `DummyPool` 是一个最小 stub：它提供与真实连接池相同的公开接口形状
///（如 `get` 与 `get_conn`），但所有方法都直接返回错误。
/// 这样可以在不引入 deadpool-postgres、tokio-postgres 等依赖的情况下，
/// 让依赖 `db::pool::DB_POOL` 的代码通过前端编译。
/// **请勿删除此 stub**，否则非 server 构建将无法通过编译。
#[cfg(not(feature = "server"))]
#[allow(dead_code)]
pub mod pool {
    /// 占位连接池，无实际数据库连接能力。
    pub struct DummyPool;

    impl DummyPool {
        /// 占位方法，永远返回错误。
        pub async fn get(&self) -> Result<(), ()> {
            Err(())
        }
    }

    /// 占位全局连接池实例。
    pub static DB_POOL: DummyPool = DummyPool;

    /// 占位函数，永远返回错误。
    pub async fn get_conn() -> Result<(), ()> {
        Err(())
    }
}

//! 数据库连接模块。
//!
//! 本模块根据 `server` feature 的启用情况提供两套实现：
//! - 启用 `server` 时，从 `pool` 子模块暴露真实的 PostgreSQL 连接池；
//! - 未启用 `server` 时（例如仅编译 WASM 前端），提供一个 `DummyPool` stub，
//!   使代码在缺少数据库依赖的情况下仍能编译通过。
//!
//! 这种 stub 模式是 Dioxus fullstack 项目的常见做法：服务端函数体在 WASM 构建时会被剥离，
//! 但模块结构必须保持一致，因此需要一个占位实现来满足编译器的符号解析。

/// 真实的 PostgreSQL 连接池实现，仅在启用 server feature 时编译。
#[cfg(feature = "server")]
pub mod pool;

/// 连接获取的指数退避重试策略，仅在启用 server feature 时编译。
#[cfg(feature = "server")]
pub mod retry;

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

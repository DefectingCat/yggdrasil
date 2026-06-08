#![allow(clippy::unused_unit)]

#[cfg(feature = "server")]
pub fn db_conn_error(e: impl std::fmt::Display) -> dioxus::prelude::ServerFnError {
    tracing::error!("DB connection failed: {}", e);
    dioxus::prelude::ServerFnError::new(format!("数据库连接失败: {}", e))
}

#[cfg(feature = "server")]
pub fn query_error(e: impl std::fmt::Display) -> dioxus::prelude::ServerFnError {
    tracing::error!("Query failed: {}", e);
    dioxus::prelude::ServerFnError::new(format!("查询失败: {}", e))
}

#[cfg(feature = "server")]
pub fn tx_error(e: impl std::fmt::Display) -> dioxus::prelude::ServerFnError {
    tracing::error!("Transaction failed: {}", e);
    dioxus::prelude::ServerFnError::new(format!("事务失败: {}", e))
}

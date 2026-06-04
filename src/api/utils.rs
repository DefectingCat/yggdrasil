#![allow(clippy::unused_unit)]

use dioxus::prelude::*;

#[cfg(feature = "server")]
pub fn db_conn_error(e: impl std::fmt::Display) -> ServerFnError {
    tracing::error!("DB connection failed: {}", e);
    ServerFnError::new(format!("数据库连接失败: {}", e))
}

#[cfg(feature = "server")]
pub fn query_error(e: impl std::fmt::Display) -> ServerFnError {
    tracing::error!("Query failed: {}", e);
    ServerFnError::new(format!("查询失败: {}", e))
}

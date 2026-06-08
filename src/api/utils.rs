#![allow(clippy::unused_unit)]

#[cfg(feature = "server")]
#[allow(dead_code)]
pub fn db_conn_error(e: impl std::fmt::Display) -> dioxus::prelude::ServerFnError {
    tracing::error!("DB connection failed: {}", e);
    dioxus::prelude::ServerFnError::new(format!("数据库连接失败: {}", e))
}

#[cfg(feature = "server")]
#[allow(dead_code)]
pub fn query_error(e: impl std::fmt::Display) -> dioxus::prelude::ServerFnError {
    tracing::error!("Query failed: {}", e);
    dioxus::prelude::ServerFnError::new(format!("查询失败: {}", e))
}

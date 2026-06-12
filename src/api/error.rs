//! 应用错误类型与 `ServerFnError` 转换。
//!
//! `AppError` 封装认证、权限、数据库、内部错误等场景，
//! 并转换为对外友好的 `ServerFnError` 消息，避免泄露 SQL 细节。

use dioxus::prelude::ServerFnError;

/// 应用层统一错误类型。
#[derive(Debug)]
pub enum AppError {
    /// 未认证（401）。
    Unauthorized(&'static str),
    /// 无权限（403）。
    Forbidden(&'static str),
    /// 资源不存在（404）。
    NotFound(&'static str),
    /// 数据库连接失败。
    DbConn(String),
    /// SQL 查询执行失败。
    Query(String),
    /// 数据库事务失败。
    Transaction(String),
    /// 内部通用错误。
    Internal(&'static str),
}

#[cfg(feature = "server")]
impl AppError {
    /// 记录并包装数据库连接错误。
    pub fn db_conn(e: impl std::fmt::Debug) -> Self {
        tracing::error!("DB connection failed: {e:?}");
        AppError::DbConn(format!("{e:?}"))
    }

    /// 记录并包装 SQL 查询错误。
    pub fn query(e: impl std::fmt::Debug) -> Self {
        tracing::error!("Query failed: {e:?}");
        AppError::Query(format!("{e:?}"))
    }

    /// 记录并包装数据库事务错误。
    pub fn tx(e: impl std::fmt::Debug) -> Self {
        tracing::error!("Transaction failed: {e:?}");
        AppError::Transaction(format!("{e:?}"))
    }
}

/// 转换为 `ServerFnError`，对数据库类错误返回通用中文提示。
impl From<AppError> for ServerFnError {
    fn from(err: AppError) -> ServerFnError {
        let msg = match &err {
            AppError::Unauthorized(m) => m.to_string(),
            AppError::Forbidden(m) => m.to_string(),
            AppError::NotFound(m) => m.to_string(),
            AppError::DbConn(_) => "服务暂时不可用".to_string(),
            AppError::Query(_) => "操作失败".to_string(),
            AppError::Transaction(_) => "操作失败".to_string(),
            AppError::Internal(m) => m.to_string(),
        };
        ServerFnError::new(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_message_passthrough() {
        let err: ServerFnError = AppError::Unauthorized("未登录").into();
        let msg = err.to_string();
        assert!(msg.contains("未登录"), "expected '未登录' in: {msg}");
    }

    #[test]
    fn db_conn_hides_internal_details() {
        let err: ServerFnError = AppError::db_conn("connection refused on port 5432").into();
        let msg = err.to_string();
        assert!(
            !msg.contains("5432"),
            "should not leak internal details: {msg}"
        );
        assert!(
            msg.contains("服务暂时不可用"),
            "expected generic message: {msg}"
        );
    }

    #[test]
    fn query_hides_sql_details() {
        let err: ServerFnError = AppError::query("syntax error at SELECT * FROM").into();
        let msg = err.to_string();
        assert!(!msg.contains("SELECT"), "should not leak SQL: {msg}");
    }

    #[test]
    fn forbidden_message_passthrough() {
        let err: ServerFnError = AppError::Forbidden("权限不足").into();
        let msg = err.to_string();
        assert!(msg.contains("权限不足"), "expected '权限不足': {msg}");
    }

    #[test]
    fn not_found_message_passthrough() {
        let err: ServerFnError = AppError::NotFound("文章不存在").into();
        let msg = err.to_string();
        assert!(msg.contains("文章不存在"), "expected passthrough: {msg}");
    }
}

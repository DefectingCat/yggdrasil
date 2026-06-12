use dioxus::prelude::ServerFnError;

#[derive(Debug)]
pub enum AppError {
    Unauthorized(&'static str),
    Forbidden(&'static str),
    NotFound(&'static str),
    DbConn(String),
    Query(String),
    Transaction(String),
    Internal(&'static str),
}

#[cfg(feature = "server")]
impl AppError {
    pub fn db_conn(e: impl std::fmt::Debug) -> Self {
        tracing::error!("DB connection failed: {e:?}");
        AppError::DbConn(format!("{e:?}"))
    }

    pub fn query(e: impl std::fmt::Debug) -> Self {
        tracing::error!("Query failed: {e:?}");
        AppError::Query(format!("{e:?}"))
    }

    pub fn tx(e: impl std::fmt::Debug) -> Self {
        tracing::error!("Transaction failed: {e:?}");
        AppError::Transaction(format!("{e:?}"))
    }
}

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

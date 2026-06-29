//! 数据导出：Axum 流式路由（大文件不走 JSON 序列化）。
//!
//! 鉴权镜像 [`crate::api::upload`]：从 cookie 取 session，校验管理员。
//! 两种来源：按表导出（表名白名单）/ 按查询导出（强制只读，AST 校验）。
//! 两种格式：CSV（COPY TO STDOUT 流式）/ SQL（逐行拼 INSERT）。

#![allow(clippy::unused_unit)]

use axum::body::Body;
use axum::extract::Query;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use futures::StreamExt;
use serde::Deserialize;

use crate::auth::session::parse_session_token;

#[derive(Deserialize)]
pub struct ExportParams {
    /// `table:<name>` 或 `query:<SELECT ...>`。
    pub source: String,
    /// `sql` 或 `csv`。
    pub format: String,
    /// 是否带表头（CSV）/ 列名（SQL INSERT）。默认 true。
    pub include_columns: Option<bool>,
}

/// POST/GET /api/database/export —— 流式导出。
pub async fn export_data(
    headers: HeaderMap,
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. 鉴权：cookie → session → admin
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let token = parse_session_token(cookie_header).map(str::to_string);
    let token = match token {
        Some(t) => t,
        None => return Err((StatusCode::UNAUTHORIZED, "未登录".to_string())),
    };
    let user = match crate::api::auth::get_user_by_token(&token).await {
        Ok(Some(u)) => u,
        _ => return Err((StatusCode::UNAUTHORIZED, "会话已过期".to_string())),
    };
    if user.role != crate::models::user::UserRole::Admin {
        return Err((StatusCode::FORBIDDEN, "权限不足".to_string()));
    }

    // 2. 解析来源 + 白名单/只读校验
    let include_columns = params.include_columns.unwrap_or(true);
    let (source_sql, table_name) = parse_source(&params.source)?;

    match params.format.as_str() {
        "csv" => export_csv(source_sql, include_columns).await,
        "sql" => export_sql(source_sql, include_columns, table_name).await,
        _ => Err((StatusCode::BAD_REQUEST, "不支持的格式".to_string())),
    }
    .map(|(body, content_type, filename)| {
        let disposition = format!("attachment; filename=\"{}\"", filename);
        let disposition_value = axum::http::HeaderValue::from_str(&disposition)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("attachment"));
        let content_type_value = axum::http::HeaderValue::from_str(content_type)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/octet-stream"));
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type_value),
                (header::CONTENT_DISPOSITION, disposition_value),
            ],
            body,
        )
    })
}

/// 解析导出来源，返回（内部 SQL，表名）。
/// - `table:posts` → 校验表名合法后，`SELECT * FROM "posts"`（只读）+ 表名 "posts"。
/// - `query:SELECT ...` → 校验为只读语句后原样返回，表名为 "export"。
fn parse_source(source: &str) -> Result<(String, String), (StatusCode, String)> {
    if let Some(table) = source.strip_prefix("table:") {
        // 表名白名单：仅允许标识符字符，防注入
        let t = table.trim();
        if t.is_empty() || !is_simple_ident(t) {
            return Err((StatusCode::BAD_REQUEST, "无效的表名".to_string()));
        }
        Ok((format!("SELECT * FROM \"{}\"", t), t.to_string()))
    } else if let Some(query) = source.strip_prefix("query:") {
        // 只读校验：sqlparser 解析后所有语句均为 Query/Explain
        let dialect = sqlparser::dialect::PostgreSqlDialect {};
        let parsed = sqlparser::parser::Parser::parse_sql(&dialect, query)
            .map_err(|_| (StatusCode::BAD_REQUEST, "SQL 解析失败".to_string()))?;
        if parsed.iter().any(|s| !is_read_only_ast(s)) {
            return Err((
                StatusCode::BAD_REQUEST,
                "导出查询必须是只读（SELECT/EXPLAIN）".to_string(),
            ));
        }
        Ok((query.to_string(), "export".to_string()))
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            "source 必须是 table:<name> 或 query:<sql>".to_string(),
        ))
    }
}

/// 简单标识符校验（字母数字下划线，防 SQL 注入与路径穿越）。
fn is_simple_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// 判断 AST 是否只读（SELECT/EXPLAIN）。
fn is_read_only_ast(stmt: &sqlparser::ast::Statement) -> bool {
    use sqlparser::ast::Statement;
    matches!(stmt, Statement::Query(_) | Statement::Explain { .. })
}

/// CSV 导出：用 COPY ... TO STDOUT WITH CSV 流式（大表不 OOM）。
async fn export_csv(
    source_sql: String,
    include_columns: bool,
) -> Result<(Body, &'static str, String), (StatusCode, String)> {
    let client = crate::db::pool::get_conn()
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "数据库不可用".to_string()))?;

    let header_clause = if include_columns { "HEADER" } else { "" };
    let copy_stmt = format!("COPY ({}) TO STDOUT WITH CSV {}", source_sql, header_clause);
    let stream = client
        .copy_out(&copy_stmt)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("COPY 失败：{e}")))?;

    // tokio_postgres 的 copy_out 流产出 Bytes，直接转 axum Body。
    let mapped = stream.map(|res| res.map_err(std::io::Error::other));
    let body = Body::from_stream(mapped);
    Ok((body, "text/csv; charset=utf-8", "export.csv".to_string()))
}

/// SQL 导出：逐行拼 INSERT（纯数据，不含 DDL）。
async fn export_sql(
    source_sql: String,
    include_columns: bool,
    table_name: String,
) -> Result<(Body, &'static str, String), (StatusCode, String)> {
    let client = crate::db::pool::get_conn()
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "数据库不可用".to_string()))?;

    let rows = client
        .query(&source_sql, &[])
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("查询失败：{e}")))?;

    let columns: Vec<String> = rows
        .first()
        .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
        .unwrap_or_default();

    let mut out = String::new();
    out.push_str("-- Yggdrasil 数据导出（仅数据，不含 schema）\n");
    let col_clause = if include_columns && !columns.is_empty() {
        format!("({})", columns.join(", "))
    } else {
        String::new()
    };

    for r in &rows {
        let vals: Vec<String> = (0..r.len())
            .map(|i| sql_quote_cell(r, i))
            .collect();
        out.push_str(&format!(
            "INSERT INTO {} {} VALUES ({});\n",
            &table_name,
            col_clause,
            vals.join(", ")
        ));
    }

    let body = Body::from(out);
    Ok((
        body,
        "application/sql; charset=utf-8",
        "export.sql".to_string(),
    ))
}

/// 把一个单元格值转成 SQL 字面量（字符串单引号转义，数字原样，NULL）。
fn sql_quote_cell(row: &tokio_postgres::Row, idx: usize) -> String {
    let ty = row.columns().get(idx).map(|c| c.type_().name()).unwrap_or("");
    match ty {
        "int2" => row
            .try_get::<_, Option<i16>>(idx)
            .ok()
            .flatten()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".into()),
        "int4" => row
            .try_get::<_, Option<i32>>(idx)
            .ok()
            .flatten()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".into()),
        "int8" => row
            .try_get::<_, Option<i64>>(idx)
            .ok()
            .flatten()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".into()),
        "float4" | "float8" => row
            .try_get::<_, Option<f64>>(idx)
            .ok()
            .flatten()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".into()),
        "bool" => row
            .try_get::<_, Option<bool>>(idx)
            .ok()
            .flatten()
            .map(|v| if v { "TRUE" } else { "FALSE" }.to_string())
            .unwrap_or_else(|| "NULL".into()),
        _ => {
            // 字符串/时间戳等：单引号转义
            match row.try_get::<_, Option<String>>(idx) {
                Ok(Some(s)) => format!("'{}'", s.replace('\'', "''")),
                _ => "NULL".into(),
            }
        }
    }
}

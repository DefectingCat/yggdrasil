#![allow(clippy::unused_unit, deprecated)]

//! SQL 控制台执行（全读写 + 4 道护栏）。
//!
//! 护栏：
//! 1. 高危语句闸门：`DROP DATABASE`/`DROP SCHEMA`（字符串预检）绝禁；
//!    `DROP`/`TRUNCATE`/`ALTER` 需 `confirm_dangerous`。
//! 2. 无 WHERE 拦截：`UPDATE`/`DELETE` 无 `selection` 拒绝。
//! 3. 查询超时上限：复用 `STATEMENT_TIMEOUT_SECS`（pool 层已注入 GUC）。
//! 4. 前端二次确认（前端实现）。
//!
//! 默认禁止多语句（`allow_multi` 放开）。

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::api::auth::get_current_admin_user;
use crate::api::error::AppError;
use crate::db::pool::get_conn;

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct ExecuteSqlOpts {
    /// 是否允许多语句（`;` 分隔），默认 false。
    pub allow_multi: bool,
    /// 是否勾选「我了解后果」（放开 DROP/TRUNCATE/ALTER 等高危）。
    pub confirm_dangerous: bool,
    /// 是否带 EXPLAIN 执行计划。
    pub with_explain: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SqlResult {
    pub columns: Vec<String>,
    /// 每格用 JSON 表示（text/int/timestamp/bool/null）。
    pub rows: Vec<Vec<serde_json::Value>>,
    pub affected_rows: u64,
    pub elapsed_ms: u64,
    /// 语句类型（来自 AST，如 "Select"/"Update"/"CreateTable"）。
    pub statement_type: String,
    pub explain: Option<String>,
    /// 是否因 500 行上限截断。
    pub truncated: bool,
    /// 截断时的估算总行数（重查用）。
    pub total_estimate: Option<i64>,
}

/// 结果行数上限（超出截断 + 提示）。
const MAX_ROWS: usize = 500;

/// 绝对禁止的语句关键词（字符串预检，sqlparser 无 ObjectType::Database/Schema）。
/// 命中即拒，不可放行。
const ABSOLUTELY_FORBIDDEN: &[&str] = &["drop database", "drop schema", "create database"];

/// 护栏检查返回值。
#[derive(Debug)]
enum GuardResult {
    Allowed,
    /// 需 confirm_dangerous 才放行。
    NeedsConfirm,
    /// 不可放行（附带原因）。
    Forbidden(String),
}

/// 护栏 1+2：sqlparser 解析后遍历 AST，检查高危语句与无 WHERE 的 UPDATE/DELETE。
#[cfg(feature = "server")]
fn check_guards(
    asts: &[sqlparser::ast::Statement],
    confirm_dangerous: bool,
) -> GuardResult {
    use sqlparser::ast::Statement;

    for stmt in asts {
        match stmt {
            // 护栏 1：需确认的高危语句
            Statement::Drop { .. } | Statement::Truncate { .. } | Statement::AlterTable { .. } => {
                if !confirm_dangerous {
                    return GuardResult::NeedsConfirm;
                }
            }
            // 护栏 2：UPDATE 无 WHERE
            Statement::Update { selection: None, .. } => {
                return GuardResult::Forbidden(
                    "UPDATE 缺少 WHERE 子句，将影响全表。请加 WHERE 条件。".to_string(),
                );
            }
            // 护栏 2：DELETE 无 WHERE
            Statement::Delete { selection: None, .. } => {
                return GuardResult::Forbidden(
                    "DELETE 缺少 WHERE 子句，将影响全表。请加 WHERE 条件。".to_string(),
                );
            }
            _ => {}
        }
    }
    GuardResult::Allowed
}

/// 提取语句类型名（AST 变体名，如 "Select"/"Insert"/"Update"）。
#[cfg(feature = "server")]
fn statement_type_name(stmt: &sqlparser::ast::Statement) -> String {
    use sqlparser::ast::Statement;
    let name = match stmt {
        Statement::Query(_) => "Select",
        Statement::Insert { .. } => "Insert",
        Statement::Update { .. } => "Update",
        Statement::Delete { .. } => "Delete",
        Statement::CreateTable { .. } => "CreateTable",
        Statement::AlterTable { .. } => "AlterTable",
        Statement::Drop { .. } => "Drop",
        Statement::Truncate { .. } => "Truncate",
        Statement::Explain { .. } => "Explain",
        _ => "Other",
    };
    name.to_string()
}

/// 判断语句是否只读（SELECT/EXPLAIN/SHOW/WITH...SELECT）。
#[cfg(feature = "server")]
fn is_read_only(stmt: &sqlparser::ast::Statement) -> bool {
    use sqlparser::ast::Statement;
    matches!(
        stmt,
        Statement::Query(_) | Statement::Explain { .. }
    )
}

/// 把一列的值转成 JSON（按 PG 类型名分发）。
#[cfg(feature = "server")]
fn col_to_json(row: &tokio_postgres::Row, idx: usize) -> serde_json::Value {
    use serde_json::json;
    let ty = row.columns().get(idx).map(|c| c.type_().name()).unwrap_or("");
    match ty {
        "int2" => row
            .try_get::<_, Option<i16>>(idx)
            .ok()
            .flatten()
            .map(|v| json!(v))
            .unwrap_or(serde_json::Value::Null),
        "int4" => row
            .try_get::<_, Option<i32>>(idx)
            .ok()
            .flatten()
            .map(|v| json!(v))
            .unwrap_or(serde_json::Value::Null),
        "int8" => row
            .try_get::<_, Option<i64>>(idx)
            .ok()
            .flatten()
            .map(|v| json!(v))
            .unwrap_or(serde_json::Value::Null),
        "float4" => row
            .try_get::<_, Option<f32>>(idx)
            .ok()
            .flatten()
            .map(|v| json!(v))
            .unwrap_or(serde_json::Value::Null),
        "float8" => row
            .try_get::<_, Option<f64>>(idx)
            .ok()
            .flatten()
            .map(|v| json!(v))
            .unwrap_or(serde_json::Value::Null),
        "bool" => row
            .try_get::<_, Option<bool>>(idx)
            .ok()
            .flatten()
            .map(|v| json!(v))
            .unwrap_or(serde_json::Value::Null),
        // 其余（text/varchar/timestamp/jsonb/...）一律按字符串取，失败则 null
        _ => row
            .try_get::<_, Option<String>>(idx)
            .ok()
            .flatten()
            .map(|v| json!(v))
            .unwrap_or(serde_json::Value::Null),
    }
}

/// 执行 SQL（全读写，管理员）。护栏见模块文档。
#[server(ExecuteSql, "/api")]
pub async fn execute_sql(sql: String, opts: ExecuteSqlOpts) -> Result<SqlResult, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        use sqlparser::dialect::PostgreSqlDialect;
        use sqlparser::parser::Parser;

        // 护栏 1（绝禁）：字符串预检 DROP/CREATE DATABASE、DROP SCHEMA
        let normalized = sql.to_lowercase();
        for forbidden in ABSOLUTELY_FORBIDDEN {
            if normalized.contains(forbidden) {
                return Err(AppError::BadRequest(format!(
                    "禁止的操作：{}",
                    forbidden.to_uppercase()
                ))
                .into());
            }
        }

        // 解析 SQL
        let dialect = PostgreSqlDialect {};
        let asts = Parser::parse_sql(&dialect, &sql)
            .map_err(|e| AppError::BadRequest(format!("SQL 解析失败：{e}")))?;
        if asts.is_empty() {
            return Err(AppError::BadRequest("空的 SQL 语句".into()).into());
        }

        // 多语句检查（默认禁止）
        if asts.len() > 1 && !opts.allow_multi {
            return Err(AppError::BadRequest(
                "检测到多条语句，请勾选「允许多语句」后再执行".into(),
            )
            .into());
        }

        // 护栏 1+2：AST 检查
        match check_guards(&asts, opts.confirm_dangerous) {
            GuardResult::Forbidden(msg) => {
                return Err(AppError::BadRequest(msg).into());
            }
            GuardResult::NeedsConfirm => {
                return Err(AppError::BadRequest(
                    "高危操作（DROP/TRUNCATE/ALTER），需勾选「我了解后果」".into(),
                )
                .into());
            }
            GuardResult::Allowed => {}
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;
        let start = std::time::Instant::now;

        // 逐条执行（allow_multi 时多条，否则单条）
        let mut last_result = SqlResult::default();
        for stmt in &asts {
            last_result = execute_one(&client, stmt, &sql, opts.with_explain, start).await?;
        }
        Ok(last_result)
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = (sql, opts);
        Ok(SqlResult::default())
    }
}

/// 执行单条语句，返回结果。
#[cfg(feature = "server")]
async fn execute_one(
    client: &deadpool_postgres::Object,
    stmt: &sqlparser::ast::Statement,
    sql: &str,
    with_explain: bool,
    start: impl Fn() -> std::time::Instant + Copy,
) -> Result<SqlResult, ServerFnError> {
    let statement_type = statement_type_name(stmt);
    let read_only = is_read_only(stmt);

    if with_explain && read_only {
        // EXPLAIN 模式：包裹原 SQL 执行计划，取首列文本拼接
        let explain_sql = format!("EXPLAIN {}", sql.trim_end_matches(';'));
        let rows = client
            .query(&explain_sql, &[])
            .await
            .map_err(AppError::query)?;
        let explain = rows
            .iter()
            .filter_map(|r| r.try_get::<_, String>(0).ok())
            .collect::<Vec<_>>()
            .join("\n");
        return Ok(SqlResult {
            statement_type,
            explain: Some(explain),
            elapsed_ms: start().elapsed().as_millis() as u64,
            ..Default::default()
        });
    }

    if read_only {
        // 只读：取结果集。列名从第一行取（空结果集时无列名，前端容错）。
        let rows = client.query(sql, &[]).await.map_err(AppError::query)?;
        let columns: Vec<String> = rows
            .first()
            .map(|r| {
                r.columns()
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect()
            })
            .unwrap_or_default();
        let mut data: Vec<Vec<serde_json::Value>> = Vec::new();
        let mut truncated = false;
        for r in &rows {
            if data.len() >= MAX_ROWS {
                truncated = true;
                break;
            }
            let row: Vec<serde_json::Value> = (0..r.len()).map(|i| col_to_json(r, i)).collect();
            data.push(row);
        }
        Ok(SqlResult {
            columns,
            rows: data,
            truncated,
            statement_type,
            elapsed_ms: start().elapsed().as_millis() as u64,
            ..Default::default()
        })
    } else {
        // 写操作：返回影响行数
        let affected = client.execute(sql, &[]).await.map_err(AppError::query)?;
        Ok(SqlResult {
            affected_rows: affected,
            statement_type,
            elapsed_ms: start().elapsed().as_millis() as u64,
            ..Default::default()
        })
    }
}

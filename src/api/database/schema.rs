#![allow(clippy::unused_unit, deprecated)]

//! SQL 补全用 schema 拉取（供 CodeMirror lang-sql 表/列补全）。

use dioxus::prelude::*;

use crate::api::auth::get_current_admin_user;
use crate::api::error::AppError;
use crate::codemirror_bridge::SqlSchema;
use crate::db::pool::get_conn;

/// 拉取数据库 schema（表名 + 列名），供 CodeMirror SQL 补全。
#[server(GetDbSchema, "/api")]
pub async fn get_db_schema() -> Result<SqlSchema, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;
        let rows = client
            .query(
                "SELECT t.table_name, \
                 string_agg(c.column_name, ',' ORDER BY c.ordinal_position) \
                 FROM information_schema.tables t \
                 JOIN information_schema.columns c USING (table_schema, table_name) \
                 WHERE t.table_schema = 'public' AND t.table_type = 'BASE TABLE' \
                 GROUP BY t.table_name ORDER BY t.table_name",
                &[],
            )
            .await
            .map_err(AppError::query)?;
        let tables = rows
            .into_iter()
            .map(|r| {
                let cols: String = r.get(1);
                crate::codemirror_bridge::SqlTable {
                    name: r.get(0),
                    columns: cols.split(',').map(|s| s.to_string()).collect(),
                }
            })
            .collect();
        Ok(SqlSchema { tables })
    }
    #[cfg(not(feature = "server"))]
    {
        Ok(SqlSchema::default())
    }
}

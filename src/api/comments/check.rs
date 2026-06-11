use dioxus::prelude::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingStatusItem {
    pub id: i64,
    pub status: String,
}

#[server(CheckPendingStatus, "/api")]
pub async fn check_pending_status(ids: Vec<i64>) -> Result<Vec<PendingStatusItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::db::pool::get_conn;
        use crate::api::error::AppError;

        if ids.is_empty() {
            return Ok(vec![]);
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let rows = client
            .query(
                "SELECT id, status FROM comments WHERE id = ANY($1)",
                &[&ids],
            )
            .await
            .map_err(AppError::query)?;

        let found: std::collections::HashMap<i64, String> = rows
            .iter()
            .map(|r| (r.get::<_, i64>(0), r.get::<_, String>(1)))
            .collect();

        let result: Vec<PendingStatusItem> = ids
            .into_iter()
            .map(|id| {
                let status = found.get(&id).cloned().unwrap_or_else(|| "gone".to_string());
                PendingStatusItem { id, status }
            })
            .collect();

        Ok(result)
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

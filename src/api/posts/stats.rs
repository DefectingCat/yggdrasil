use dioxus::prelude::*;

use super::helpers::get_current_admin_user;
use super::types::PostStatsResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;
use crate::models::post::PostStats;

#[server(GetPostStats, "/api")]
pub async fn get_post_stats() -> Result<PostStatsResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        if let Some(cached) = crate::cache::get_post_stats().await {
            return Ok(PostStatsResponse { stats: cached });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let total: i64 = client
            .query_one("SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL", &[])
            .await
            .map_err(AppError::query)?
            .get(0);

        let drafts: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL AND status = 'draft'",
                &[],
            )
            .await
            .map_err(AppError::query)?
            .get(0);

        let published: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL AND status = 'published'",
                &[],
            )
            .await
            .map_err(AppError::query)?
            .get(0);

        let stats = PostStats {
            total,
            drafts,
            published,
        };
        crate::cache::set_post_stats(stats.clone()).await;
        Ok(PostStatsResponse { stats })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(PostStatsResponse {
            stats: PostStats {
                total: 0,
                drafts: 0,
                published: 0,
            },
        })
    }
}

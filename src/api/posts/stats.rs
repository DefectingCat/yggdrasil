//! 文章统计接口。
//!
//! 返回文章总数、草稿数与已发布数，供管理后台仪表盘使用，结果缓存。
//! Dioxus server function，注册在 `/api` 路径下。
//! 仅在 `feature = "server"` 启用的服务端构建中查询数据库。

use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::get_current_admin_user;
use super::types::PostStatsResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;
use crate::models::post::PostStats;

/// 获取文章统计信息。
///
/// 需要 admin 权限；优先命中缓存，未命中时分别统计总数、草稿数与已发布数。
#[server(GetPostStats, "/api")]
pub async fn get_post_stats() -> Result<PostStatsResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        if let Some(cached) = crate::cache::get_post_stats().await {
            return Ok(PostStatsResponse { stats: cached });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        // 统计未删除文章总数。
        let total: i64 = client
            .query_one("SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL", &[])
            .await
            .map_err(AppError::query)?
            .get(0);

        // 统计草稿数量。
        let drafts: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL AND status = 'draft'",
                &[],
            )
            .await
            .map_err(AppError::query)?
            .get(0);

        // 统计已发布数量。
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

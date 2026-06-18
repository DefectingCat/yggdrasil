//! 删除文章接口。
//!
//! 采用软删除方式，将 posts.deleted_at 设置为当前时间，
//! 并按影响范围失效相关缓存。
//! Dioxus server function，注册在 `/api` 路径下。
//! 仅在 `feature = "server"` 启用的服务端构建中执行删除与缓存失效。

use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::get_current_admin_user;
use super::types::CreatePostResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
#[cfg(feature = "server")]
use crate::db::pool::get_conn;

/// 删除指定文章。
///
/// 仅 admin 可调用；通过设置 deleted_at 实现软删除，
/// 成功后按影响范围失效文章列表、标签云、统计、slug 及相关标签文章缓存。
#[server(DeletePost, "/api")]
pub async fn delete_post(post_id: i32) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let mut client = get_conn().await.map_err(AppError::db_conn)?;
        let tx = client.transaction().await.map_err(AppError::tx)?;

        // 在事务内锁定行并读取 slug，避免并发更新导致缓存失效目标过期。
        let slug_row = tx
            .query_opt(
                "SELECT slug FROM posts WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        let Some(slug_row) = slug_row else {
            return Ok(CreatePostResponse {
                success: false,
                message: "文章不存在".to_string(),
                post_id: None,
                slug: None,
            });
        };
        let slug: String = slug_row.get(0);

        let tag_rows = tx
            .query(
                "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;
        let tags: Vec<String> = tag_rows.iter().map(|r| r.get(0)).collect();

        // 软删除：仅影响未被删除的文章。
        let result = tx
            .execute(
                "UPDATE posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
                &[&post_id],
            )
            .await
            .map_err(AppError::tx)?;

        if result == 0 {
            return Ok(CreatePostResponse {
                success: false,
                message: "文章不存在".to_string(),
                post_id: None,
                slug: None,
            });
        }

        tx.commit().await.map_err(AppError::tx)?;

        // 删除后按影响范围精准失效缓存。
        crate::cache::invalidate_post_lists();
        crate::cache::invalidate_all_tags();
        crate::cache::invalidate_post_stats();
        crate::cache::invalidate_search_results();
        crate::cache::invalidate_post_by_slug(&slug).await;
        crate::cache::invalidate_tag_posts_for(&tags).await;

        // 递增 SSR 全局世代号（未来就绪基础设施；当前不会使 Dioxus 0.7 SSR 缓存失效）。
        crate::ssr_cache::bump_global_generation();

        Ok(CreatePostResponse {
            success: true,
            message: "删除成功".to_string(),
            post_id: Some(post_id),
            slug: Some(slug),
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(CreatePostResponse {
            success: false,
            message: "server only".to_string(),
            post_id: None,
            slug: None,
        })
    }
}

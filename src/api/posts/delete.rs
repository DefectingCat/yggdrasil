//! 删除文章接口。
//!
//! 采用软删除方式，将 posts.deleted_at 设置为当前时间，
//! 同时清空所有文章相关缓存。
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
/// 成功后清空全部文章缓存。
#[server(DeletePost, "/api")]
pub async fn delete_post(post_id: i32) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;

        // 软删除：仅影响未被删除的文章。
        let result = client
            .execute(
                "UPDATE posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        if result == 0 {
            return Ok(CreatePostResponse {
                success: false,
                message: "文章不存在".to_string(),
                post_id: None,
                slug: None,
            });
        }

        // 删除后所有文章相关缓存均失效。
        crate::cache::invalidate_all_post_caches();

        Ok(CreatePostResponse {
            success: true,
            message: "删除成功".to_string(),
            post_id: Some(post_id),
            slug: None,
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

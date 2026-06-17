//! 回收站操作接口：恢复、彻底删除、批量操作与一键清空。
//!
//! 所有接口需要 admin 权限，操作后清空全部文章相关缓存。
//! Dioxus server function，注册在 `/api` 路径下。
//! 仅在 `feature = "server"` 启用的服务端构建中执行数据库操作。

use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::get_current_admin_user;
use super::types::CreatePostResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
#[cfg(feature = "server")]
use crate::api::slug::ensure_unique_slug;
#[cfg(feature = "server")]
use crate::db::pool::get_conn;

/// 恢复一篇已删除的文章（将 deleted_at 置空）。
///
/// 若该文章原始 slug 已被其他未删除文章占用，自动追加数字后缀。
#[server(RestorePost, "/api")]
pub async fn restore_post(post_id: i32) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let mut client = get_conn().await.map_err(AppError::db_conn)?;
        let tx = client.transaction().await.map_err(AppError::tx)?;

        // 读取待恢复文章的当前 slug 与是否确已删除。
        let row = tx
            .query_opt(
                "SELECT slug FROM posts WHERE id = $1 AND deleted_at IS NOT NULL",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        let Some(row) = row else {
            return Ok(CreatePostResponse {
                success: false,
                message: "文章不在回收站".to_string(),
                post_id: None,
                slug: None,
            });
        };

        let current_slug: String = row.get("slug");

        // 恢复时确保 slug 在未删除文章中唯一（自动加后缀）；在事务内检查避免并发竞态。
        let new_slug = ensure_unique_slug(&tx, &current_slug, Some(post_id)).await?;

        // 置空 deleted_at，并更新 slug（可能已加后缀）。
        let result = tx
            .execute(
                "UPDATE posts SET deleted_at = NULL, slug = $1 WHERE id = $2 AND deleted_at IS NOT NULL",
                &[&new_slug, &post_id],
            )
            .await
            .map_err(AppError::tx)?;

        if result == 0 {
            return Ok(CreatePostResponse {
                success: false,
                message: "文章不在回收站".to_string(),
                post_id: None,
                slug: None,
            });
        }

        tx.commit().await.map_err(AppError::tx)?;

        crate::cache::invalidate_all_post_caches();

        Ok(CreatePostResponse {
            success: true,
            message: "恢复成功".to_string(),
            post_id: Some(post_id),
            slug: Some(new_slug),
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

/// 彻底删除一篇已删除的文章（物理删除，不可恢复）。
///
/// 注意：仅删除数据库记录，不删除已上传的图片文件。
/// post_tags 关联因外键 ON DELETE CASCADE 自动清理。
#[server(PurgePost, "/api")]
pub async fn purge_post(post_id: i32) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let result = client
            .execute(
                "DELETE FROM posts WHERE id = $1 AND deleted_at IS NOT NULL",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        if result == 0 {
            return Ok(CreatePostResponse {
                success: false,
                message: "文章不在回收站".to_string(),
                post_id: None,
                slug: None,
            });
        }

        crate::cache::invalidate_all_post_caches();

        Ok(CreatePostResponse {
            success: true,
            message: "彻底删除成功".to_string(),
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

/// 批量恢复文章。
#[server(BatchRestorePosts, "/api")]
pub async fn batch_restore_posts(post_ids: Vec<i32>) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        if post_ids.is_empty() {
            return Ok(CreatePostResponse {
                success: true,
                message: "无操作".to_string(),
                post_id: None,
                slug: None,
            });
        }

        let mut client = get_conn().await.map_err(AppError::db_conn)?;
        let tx = client.transaction().await.map_err(AppError::tx)?;

        // 逐条恢复，slug 冲突时自动加后缀。
        let mut restored = 0u64;
        for id in &post_ids {
            let row = tx
                .query_opt(
                    "SELECT slug FROM posts WHERE id = $1 AND deleted_at IS NOT NULL",
                    &[&id],
                )
                .await
                .map_err(AppError::query)?;
            if let Some(row) = row {
                let current_slug: String = row.get("slug");
                let new_slug = ensure_unique_slug(&tx, &current_slug, Some(*id)).await?;
                let n = tx
                    .execute(
                        "UPDATE posts SET deleted_at = NULL, slug = $1 WHERE id = $2 AND deleted_at IS NOT NULL",
                        &[&new_slug, &id],
                    )
                    .await
                    .map_err(AppError::tx)?;
                restored += n;
            }
        }

        tx.commit().await.map_err(AppError::tx)?;

        crate::cache::invalidate_all_post_caches();

        Ok(CreatePostResponse {
            success: true,
            message: format!("已恢复 {restored} 篇"),
            post_id: None,
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

/// 批量彻底删除文章。
#[server(BatchPurgePosts, "/api")]
pub async fn batch_purge_posts(post_ids: Vec<i32>) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        if post_ids.is_empty() {
            return Ok(CreatePostResponse {
                success: true,
                message: "无操作".to_string(),
                post_id: None,
                slug: None,
            });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let total = post_ids.len() as i64;
        let result = client
            .execute(
                "DELETE FROM posts WHERE id = ANY($1) AND deleted_at IS NOT NULL",
                &[&post_ids],
            )
            .await
            .map_err(AppError::query)?;

        crate::cache::invalidate_all_post_caches();

        Ok(CreatePostResponse {
            success: true,
            message: format!("已彻底删除 {result}/{total} 篇"),
            post_id: None,
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

/// 清空回收站：彻底删除所有已软删除的文章。
#[server(EmptyTrash, "/api")]
pub async fn empty_trash() -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let result = client
            .execute("DELETE FROM posts WHERE deleted_at IS NOT NULL", &[])
            .await
            .map_err(AppError::query)?;

        crate::cache::invalidate_all_post_caches();

        Ok(CreatePostResponse {
            success: true,
            message: format!("已清空回收站（{result} 篇）"),
            post_id: None,
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

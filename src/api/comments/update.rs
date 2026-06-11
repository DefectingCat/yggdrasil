use dioxus::prelude::*;
use crate::api::comments::types::*;

#[server(ApproveComment, "/api")]
pub async fn approve_comment(id: i64) -> Result<CommentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::cache;
        use crate::db::pool::get_conn;
        use crate::api::error::AppError;
        use crate::api::auth::get_current_admin_user;

        let _admin = get_current_admin_user().await?;

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let row = client
            .query_opt(
                "SELECT post_id, status FROM comments WHERE id = $1 AND deleted_at IS NULL",
                &[&id],
            )
            .await
            .map_err(AppError::query)?;

        let post_id: i32 = match row {
            Some(r) => r.get("post_id"),
            None => {
                return Ok(CommentResponse {
                    success: false,
                    message: "评论不存在".to_string(),
                    error_code: Some("not_found".into()),
                });
            }
        };

        client
            .execute(
                "UPDATE comments SET status = 'approved', approved_at = NOW() WHERE id = $1",
                &[&id],
            )
            .await
            .map_err(AppError::query)?;

        client
            .execute(
                "WITH RECURSIVE ancestors AS ( \
                     SELECT parent_id FROM comments WHERE id = $1 \
                     UNION ALL \
                     SELECT c.parent_id FROM comments c JOIN ancestors a ON c.id = a.parent_id WHERE a.parent_id IS NOT NULL \
                 ) \
                 UPDATE comments SET status = 'approved', approved_at = NOW() \
                 WHERE id IN (SELECT parent_id FROM ancestors WHERE parent_id IS NOT NULL) AND status = 'pending'",
                &[&id],
            )
            .await
            .map_err(AppError::query)?;

        cache::invalidate_comments_by_post(post_id).await;
        cache::invalidate_comment_count(post_id).await;
        cache::invalidate_pending_count().await;

        Ok(CommentResponse {
            success: true,
            message: "已通过".to_string(),
            error_code: None,
        })
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

#[server(SpamComment, "/api")]
pub async fn spam_comment(id: i64) -> Result<CommentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::cache;
        use crate::db::pool::get_conn;
        use crate::api::error::AppError;
        use crate::api::auth::get_current_admin_user;

        let _admin = get_current_admin_user().await?;

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let row = client
            .query_opt(
                "SELECT post_id, status FROM comments WHERE id = $1 AND deleted_at IS NULL",
                &[&id],
            )
            .await
            .map_err(AppError::query)?;

        if let Some(r) = row {
            let post_id: i32 = r.get("post_id");
            let old_status: String = r.get("status");

            client
                .execute(
                    "UPDATE comments SET status = 'spam' WHERE id = $1 AND deleted_at IS NULL",
                    &[&id],
                )
                .await
                .map_err(AppError::query)?;

            if old_status == "approved" {
                cache::invalidate_comments_by_post(post_id).await;
                cache::invalidate_comment_count(post_id).await;
            }
            cache::invalidate_pending_count().await;
        }

        Ok(CommentResponse {
            success: true,
            message: "已标记为垃圾".to_string(),
            error_code: None,
        })
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

#[server(TrashComment, "/api")]
pub async fn trash_comment(id: i64) -> Result<CommentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::cache;
        use crate::db::pool::get_conn;
        use crate::api::error::AppError;
        use crate::api::auth::get_current_admin_user;

        let _admin = get_current_admin_user().await?;

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let row = client
            .query_opt(
                "SELECT post_id FROM comments WHERE id = $1 AND deleted_at IS NULL",
                &[&id],
            )
            .await
            .map_err(AppError::query)?;

        if let Some(r) = row {
            let post_id: i32 = r.get("post_id");

            client
                .execute(
                    "UPDATE comments SET status = 'trash', deleted_at = NOW() WHERE id = $1",
                    &[&id],
                )
                .await
                .map_err(AppError::query)?;

            cache::invalidate_comments_by_post(post_id).await;
            cache::invalidate_comment_count(post_id).await;
            cache::invalidate_pending_count().await;
        }

        Ok(CommentResponse {
            success: true,
            message: "已删除".to_string(),
            error_code: None,
        })
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

#[server(BatchUpdateCommentStatus, "/api")]
pub async fn batch_update_comment_status(
    ids: Vec<i64>,
    status: String,
) -> Result<BatchStatusResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::cache;
        use crate::db::pool::get_conn;
        use crate::api::error::AppError;
        use crate::api::auth::get_current_admin_user;

        let _admin = get_current_admin_user().await?;

        if !matches!(status.as_str(), "approved" | "spam" | "trash") {
            return Ok(BatchStatusResponse {
                success: false,
                updated_count: 0,
                message: "无效的状态".to_string(),
            });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let post_ids: Vec<i32> = client
            .query(
                "SELECT DISTINCT post_id FROM comments WHERE id = ANY($1)",
                &[&ids],
            )
            .await
            .map_err(AppError::query)?
            .iter()
            .map(|r| r.get("post_id"))
            .collect();

        let result = if status == "trash" {
            client
                .execute(
                    "UPDATE comments SET status = $1, deleted_at = NOW() WHERE id = ANY($2)",
                    &[&status, &ids],
                )
                .await
                .map_err(AppError::query)?
        } else if status == "approved" {
            client
                .execute(
                    "UPDATE comments SET status = $1, approved_at = NOW() WHERE id = ANY($2)",
                    &[&status, &ids],
                )
                .await
                .map_err(AppError::query)?
        } else {
            client
                .execute(
                    "UPDATE comments SET status = $1 WHERE id = ANY($2)",
                    &[&status, &ids],
                )
                .await
                .map_err(AppError::query)?
        };

        cache::invalidate_pending_count().await;
        for pid in post_ids {
            cache::invalidate_comments_by_post(pid).await;
            cache::invalidate_comment_count(pid).await;
        }

        Ok(BatchStatusResponse {
            success: true,
            updated_count: result as i64,
            message: format!("已更新 {} 条评论", result),
        })
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

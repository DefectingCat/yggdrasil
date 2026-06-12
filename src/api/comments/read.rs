use crate::api::comments::types::*;
use dioxus::prelude::*;

#[server(GetComments, "/api")]
pub async fn get_comments(post_id: i32) -> Result<CommentTreeResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::api::comments::helpers::row_to_public_comment;
        use crate::api::error::AppError;
        use crate::cache;
        use crate::db::pool::get_conn;

        if let Some(cached) = cache::get_comments_by_post(post_id).await {
            let count = cached.len() as i64;
            return Ok(CommentTreeResponse {
                comments: cached,
                count,
            });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let rows = client
            .query(
                "SELECT id, parent_id, depth, author_name, author_email, author_url, content_html, created_at \
                 FROM comments \
                 WHERE post_id = $1 AND status = 'approved' AND deleted_at IS NULL \
                 ORDER BY id ASC",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        let comments: Vec<_> = rows.iter().map(row_to_public_comment).collect();
        let count = comments.len() as i64;

        cache::set_comments_by_post(post_id, comments.clone()).await;

        Ok(CommentTreeResponse { comments, count })
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

#[server(GetCommentCount, "/api")]
pub async fn get_comment_count(post_id: i32) -> Result<CommentCountResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::api::error::AppError;
        use crate::cache;
        use crate::db::pool::get_conn;

        if let Some(cached) = cache::get_comment_count(post_id).await {
            return Ok(CommentCountResponse { count: cached });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let count: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM comments WHERE post_id = $1 AND status = 'approved' AND deleted_at IS NULL",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?
            .get(0);

        cache::set_comment_count(post_id, count).await;

        Ok(CommentCountResponse { count })
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

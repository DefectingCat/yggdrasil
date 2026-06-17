//! 前端评论读取接口：已审核评论列表。
//!
//! 结果按文章 id 缓存，Dioxus server function 注册在 `/api` 路径下。
//! 仅在 `feature = "server"` 启用的服务端构建中查询数据库。

use crate::api::comments::types::*;
use dioxus::prelude::*;

/// 获取指定文章的已审核评论列表。
///
/// 优先命中缓存；按 id 升序返回，便于前端构建嵌套树。
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
                   AND EXISTS (SELECT 1 FROM posts p WHERE p.id = $1 AND p.status = 'published' AND p.deleted_at IS NULL) \
                 ORDER BY id ASC \
                 LIMIT 200",
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


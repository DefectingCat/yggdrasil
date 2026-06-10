use dioxus::prelude::*;

use super::helpers::row_to_post_list;
use super::types::PostListResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;

#[server(SearchPosts, "/api")]
pub async fn search_posts(query: String) -> Result<PostListResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let q = query.trim();
        if q.is_empty() {
            return Ok(PostListResponse { posts: Vec::new(), total: 0 });
        }

        let rows = client
            .query(
                "SELECT 
                    p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                    p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                    COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as tags,
                    word_similarity(p.search_text, $1) AS sml
                 FROM posts p
                 LEFT JOIN post_tags pt ON p.id = pt.post_id
                 LEFT JOIN tags t ON pt.tag_id = t.id
                 WHERE p.status = 'published' AND p.deleted_at IS NULL
                   AND p.search_text ILIKE '%' || $1 || '%'
                 GROUP BY p.id, p.search_text
                 ORDER BY sml DESC, p.published_at DESC
                 LIMIT 50",
                &[&q],
            )
            .await
            .map_err(AppError::query)?;

        let mut posts = Vec::new();
        for row in &rows {
            posts.push(row_to_post_list(&client, row).await);
        }

        let total = posts.len() as i64;
        Ok(PostListResponse { posts, total })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(PostListResponse { posts: Vec::new(), total: 0 })
    }
}

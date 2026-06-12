//! 文章全文搜索接口。
//!
//! 基于 PostgreSQL 的 pg_trgm 扩展，通过 word_similarity 对 search_text 做模糊匹配，
//! 按相似度与发布时间降序返回最多 50 篇已发布文章。
//! Dioxus server function，注册在 `/api` 路径下。
//! 仅在 `feature = "server"` 启用的服务端构建中查询数据库。

use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::row_to_post_list;
use super::types::PostListResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;

/// 搜索已发布文章。
///
/// 空查询直接返回空结果；非空查询使用 `word_similarity` 计算相关度，
/// 并限制返回 50 条记录。当前未缓存，每次均查询数据库。
#[server(SearchPosts, "/api")]
pub async fn search_posts(query: String) -> Result<PostListResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let q = query.trim();
        if q.is_empty() {
            return Ok(PostListResponse {
                posts: Vec::new(),
                total: 0,
            });
        }

        // 使用 ILIKE 做前缀模糊匹配，并按 word_similarity 降序、发布时间降序排序。
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
        Ok(PostListResponse {
            posts: Vec::new(),
            total: 0,
        })
    }
}

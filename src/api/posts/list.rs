use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::{get_current_admin_user, row_to_post_list};
use super::types::PostListResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;

#[server(ListPublishedPosts, "/api")]
pub async fn list_published_posts(
    page: i32,
    per_page: i32,
) -> Result<PostListResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let cache_key = crate::cache::CacheKey::PublishedPosts { page, per_page };
        if let Some((cached_posts, cached_total)) = crate::cache::get_post_list(&cache_key).await {
            return Ok(PostListResponse { posts: cached_posts, total: cached_total });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        // Get total count from cache or query
        let total = if let Some(cached_total) = crate::cache::get_total_published_posts().await {
            cached_total
        } else {
            let count_row = client
                .query_one(
                    "SELECT COUNT(*) FROM posts WHERE status = 'published' AND deleted_at IS NULL",
                    &[],
                )
                .await
                .map_err(AppError::query)?;
            let total: i64 = count_row.get(0);
            crate::cache::set_total_published_posts(total).await;
            total
        };

        let offset = ((page - 1).max(0) as i64) * (per_page as i64);
        let limit = per_page as i64;
        let rows = client
            .query(
                "SELECT 
                    p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                    p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                    COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as tags
                 FROM posts p
                 LEFT JOIN post_tags pt ON p.id = pt.post_id
                 LEFT JOIN tags t ON pt.tag_id = t.id
                 WHERE p.status = 'published' AND p.deleted_at IS NULL
                 GROUP BY p.id
                 ORDER BY p.published_at DESC
                 LIMIT $1 OFFSET $2",
                &[&limit, &offset],
            )
            .await
            .map_err(AppError::query)?;

        let mut posts = Vec::new();
        for row in &rows {
            posts.push(row_to_post_list(&client, row).await);
        }

        crate::cache::set_post_list(&cache_key, posts.clone(), total).await;
        Ok(PostListResponse { posts, total })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(PostListResponse { posts: Vec::new(), total: 0 })
    }
}

#[server(ListPosts, "/api")]
pub async fn list_posts(
    page: i32,
    per_page: i32,
) -> Result<PostListResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let count_row = client
            .query_one(
                "SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL",
                &[],
            )
            .await
            .map_err(AppError::query)?;
        let total: i64 = count_row.get(0);

        let offset = ((page - 1).max(0) as i64) * (per_page as i64);
        let limit = per_page as i64;
        let rows = client
            .query(
                "SELECT 
                    p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                    p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                    COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as tags
                 FROM posts p
                 LEFT JOIN post_tags pt ON p.id = pt.post_id
                 LEFT JOIN tags t ON pt.tag_id = t.id
                 WHERE p.deleted_at IS NULL
                 GROUP BY p.id
                 ORDER BY p.created_at DESC
                 LIMIT $1 OFFSET $2",
                &[&limit, &offset],
            )
            .await
            .map_err(AppError::query)?;

        let mut posts = Vec::new();
        for row in &rows {
            posts.push(row_to_post_list(&client, row).await);
        }

        Ok(PostListResponse { posts, total })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(PostListResponse { posts: Vec::new(), total: 0 })
    }
}

#[server(GetPostsByTag, "/api")]
pub async fn get_posts_by_tag(tag_name: String) -> Result<PostListResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if let Some((cached_posts, cached_total)) = crate::cache::get_posts_by_tag(&tag_name).await {
            return Ok(PostListResponse { posts: cached_posts, total: cached_total });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let rows = client
            .query(
                "SELECT 
                    p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                    p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                    COALESCE(array_agg(t2.name) FILTER (WHERE t2.name IS NOT NULL), '{}') as tags
                 FROM posts p
                 JOIN post_tags pt ON p.id = pt.post_id
                 JOIN tags t ON pt.tag_id = t.id
                 LEFT JOIN post_tags pt2 ON p.id = pt2.post_id
                 LEFT JOIN tags t2 ON pt2.tag_id = t2.id
                 WHERE t.name = $1 AND p.status = 'published' AND p.deleted_at IS NULL
                 GROUP BY p.id
                 ORDER BY p.published_at DESC",
                &[&tag_name],
            )
            .await
            .map_err(AppError::query)?;

        let mut posts = Vec::new();
        for row in &rows {
            posts.push(row_to_post_list(&client, row).await);
        }

        // NOTE: total = posts.len() is correct because get_posts_by_tag
        // currently fetches ALL matching posts (no LIMIT/OFFSET).
        // If pagination is added later, switch to a proper COUNT(*) query.
        let total = posts.len() as i64;
        crate::cache::set_posts_by_tag(&tag_name, posts.clone(), total).await;
        Ok(PostListResponse { posts, total })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(PostListResponse { posts: Vec::new(), total: 0 })
    }
}

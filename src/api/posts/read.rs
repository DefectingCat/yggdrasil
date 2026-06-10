use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::{get_current_admin_user, row_to_post_full, row_to_post_list};
use super::types::SinglePostResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;

#[server(GetPostById, "/api")]
pub async fn get_post_by_id(post_id: i32) -> Result<SinglePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let row = client
            .query_opt(
                "SELECT 
                    p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                    p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                    COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as tags
                 FROM posts p
                 LEFT JOIN post_tags pt ON p.id = pt.post_id
                 LEFT JOIN tags t ON pt.tag_id = t.id
                 WHERE p.id = $1 AND p.deleted_at IS NULL
                 GROUP BY p.id",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        let post = match row {
            Some(row) => Some(row_to_post_list(&client, &row).await),
            None => None,
        };

        Ok(SinglePostResponse { post })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(SinglePostResponse { post: None })
    }
}

#[server(GetPostBySlug, "/api")]
pub async fn get_post_by_slug(slug: String) -> Result<SinglePostResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if let Some(cached) = crate::cache::get_post_by_slug(&slug).await {
            return Ok(SinglePostResponse { post: cached });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let row = client
            .query_opt(
                "SELECT 
                    p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                    p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                    COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as tags,
                    prev.title as prev_title, prev.slug as prev_slug,
                    next.title as next_title, next.slug as next_slug
                 FROM posts p
                 LEFT JOIN post_tags pt ON p.id = pt.post_id
                 LEFT JOIN tags t ON pt.tag_id = t.id
                 LEFT JOIN LATERAL (
                     SELECT title, slug FROM posts 
                     WHERE published_at < p.published_at 
                       AND status = 'published' 
                       AND deleted_at IS NULL
                     ORDER BY published_at DESC
                     LIMIT 1
                 ) prev ON true
                 LEFT JOIN LATERAL (
                     SELECT title, slug FROM posts 
                     WHERE published_at > p.published_at 
                       AND status = 'published' 
                       AND deleted_at IS NULL
                     ORDER BY published_at ASC
                     LIMIT 1
                 ) next ON true
                 WHERE p.slug = $1 AND p.deleted_at IS NULL
                 GROUP BY p.id, prev.title, prev.slug, next.title, next.slug",
                &[&slug],
            )
            .await
            .map_err(AppError::query)?;

        let post = match row {
            Some(row) => Some(row_to_post_full(&client, &row).await),
            None => None,
        };

        if post.is_some() {
            crate::cache::set_post_by_slug(&slug, post.clone()).await;
        }
        Ok(SinglePostResponse { post })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(SinglePostResponse { post: None })
    }
}

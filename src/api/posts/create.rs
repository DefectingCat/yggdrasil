#![allow(clippy::too_many_arguments)]

use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::{clean_tags, get_current_admin_user, sync_tags};
use super::types::CreatePostResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;
use crate::models::post::PostStatus;

#[server(CreatePost, "/api")]
pub async fn create_post(
    title: String,
    slug: Option<String>,
    summary: Option<String>,
    content_md: String,
    status: String,
    tags: Vec<String>,
    cover_image: Option<String>,
) -> Result<CreatePostResponse, ServerFnError> {
    let user = get_current_admin_user().await?;

    if title.trim().is_empty() {
        return Ok(CreatePostResponse {
            success: false,
            message: "标题不能为空".to_string(),
            post_id: None,
            slug: None,
        });
    }

    if content_md.trim().is_empty() {
        return Ok(CreatePostResponse {
            success: false,
            message: "内容不能为空".to_string(),
            post_id: None,
            slug: None,
        });
    }

    let base_slug = match slug {
        Some(ref s) if !s.trim().is_empty() => {
            let s = s.trim();
            if !crate::api::slug::is_valid_slug(s) {
                return Ok(CreatePostResponse {
                    success: false,
                    message: "slug 格式无效，只能包含字母、数字、连字符和下划线".to_string(),
                    post_id: None,
                    slug: None,
                });
            }
            s.to_string()
        }
        _ => crate::api::slug::slugify(&title),
    };

    #[cfg(feature = "server")]
    {
        let mut client = get_conn().await.map_err(AppError::db_conn)?;

        let final_slug = crate::api::slug::ensure_unique_slug(&client, &base_slug, None).await?;
        let rendered = crate::api::markdown::render_markdown_enhanced(&content_md);
        let content_html = rendered.html;
        let toc_html = if rendered.toc_html.is_empty() {
            None::<String>
        } else {
            Some(rendered.toc_html)
        };
        let summary = summary
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| crate::utils::text::auto_summary(&content_md));
        let post_status = PostStatus::from_str(&status).unwrap_or(PostStatus::Draft);
        let cover_image = cover_image.filter(|s| !s.trim().is_empty());

        let published_at = if post_status == PostStatus::Published {
            Some(chrono::Utc::now())
        } else {
            None
        };

        let tx = client.transaction().await.map_err(AppError::tx)?;

        let row = tx
            .query_one(
                "INSERT INTO posts (author_id, title, slug, summary, content_md, content_html, toc_html, status, published_at, cover_image)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 RETURNING id",
                &[
                    &user.id,
                    &title.trim(),
                    &final_slug,
                    &summary,
                    &content_md,
                    &content_html,
                    &toc_html,
                    &post_status.as_str(),
                    &published_at,
                    &cover_image,
                ],
            )
            .await
            .map_err(AppError::tx)?;

        let post_id: i32 = row.get(0);

        let tags_cleaned = clean_tags(&tags);
        sync_tags(&tx, post_id, &tags_cleaned).await?;

        tx.commit().await.map_err(AppError::tx)?;

        crate::cache::invalidate_post_lists();
        crate::cache::invalidate_all_tags();
        crate::cache::invalidate_post_stats();

        for tag_name in &tags_cleaned {
            crate::cache::invalidate_posts_by_tag(tag_name).await;
        }

        Ok(CreatePostResponse {
            success: true,
            message: "创建成功".to_string(),
            post_id: Some(post_id),
            slug: Some(final_slug),
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

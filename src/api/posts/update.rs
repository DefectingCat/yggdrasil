use dioxus::prelude::*;

use super::helpers::{clean_tags, get_current_admin_user, sync_tags};
use super::types::CreatePostResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;
use crate::models::post::PostStatus;

#[server(UpdatePost, "/api")]
pub async fn update_post(
    post_id: i32,
    title: String,
    slug: Option<String>,
    summary: Option<String>,
    content_md: String,
    status: String,
    tags: Vec<String>,
    cover_image: Option<String>,
) -> Result<CreatePostResponse, ServerFnError> {
    let user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let mut client = get_conn().await.map_err(AppError::db_conn)?;

        let old_slug: Option<String> = client
            .query_opt("SELECT slug FROM posts WHERE id = $1", &[&post_id])
            .await
            .map_err(AppError::query)?
            .map(|r| r.get(0));

        let exists: bool = client
            .query_opt(
                "SELECT 1 FROM posts WHERE id = $1 AND author_id = $2 AND deleted_at IS NULL",
                &[&post_id, &user.id],
            )
            .await
            .map_err(AppError::query)?
            .is_some();

        if !exists {
            return Ok(CreatePostResponse {
                success: false,
                message: "文章不存在或无权限".to_string(),
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
                        message: "slug 格式无效".to_string(),
                        post_id: None,
                        slug: None,
                    });
                }
                s.to_string()
            }
            _ => crate::api::slug::slugify(&title),
        };

        let final_slug = crate::api::slug::ensure_unique_slug(&client, &base_slug, Some(post_id)).await?;
        let rendered = crate::api::markdown::render_markdown_enhanced(&content_md);
        let content_html = rendered.html;
        let summary = summary
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| crate::utils::text::auto_summary(&content_md));
        let post_status = PostStatus::from_str(&status).unwrap_or(PostStatus::Draft);
        let cover_image = cover_image.filter(|s| !s.trim().is_empty());

        let tx = client.transaction().await.map_err(AppError::tx)?;

        let old_tags: Vec<String> = {
            let rows = tx
                .query(
                    "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1",
                    &[&post_id],
                )
                .await
                .map_err(AppError::query)?;
            rows.iter().map(|r| r.get(0)).collect()
        };

        let old_status_row = tx
            .query_opt(
                "SELECT status, published_at FROM posts WHERE id = $1",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        let published_at = if post_status == PostStatus::Published {
            let was_published = old_status_row
                .as_ref()
                .map(|r| {
                    let s: String = r.get(0);
                    s == "published"
                })
                .unwrap_or(false);
            let existing_published: Option<chrono::DateTime<chrono::Utc>> =
                old_status_row.as_ref().and_then(|r| r.get(1));

            if was_published {
                existing_published
            } else {
                Some(chrono::Utc::now())
            }
        } else {
            old_status_row.and_then(|r| r.get(1))
        };

        tx.execute(
            "UPDATE posts SET title = $1, slug = $2, summary = $3, content_md = $4, content_html = $5, status = $6, published_at = $7, cover_image = $8, updated_at = NOW()
             WHERE id = $9",
            &[
                &title.trim(),
                &final_slug,
                &summary,
                &content_md,
                &content_html,
                &post_status.as_str(),
                &published_at,
                &cover_image,
                &post_id,
            ],
        )
        .await
        .map_err(AppError::tx)?;

        let tags_cleaned = clean_tags(&tags);
        let tags_for_invalidation = tags_cleaned.clone();

        tx.execute("DELETE FROM post_tags WHERE post_id = $1", &[&post_id])
            .await
            .map_err(AppError::tx)?;

        sync_tags(&tx, post_id, &tags_cleaned).await?;

        tx.commit().await.map_err(AppError::tx)?;

        crate::cache::invalidate_post_lists();
        crate::cache::invalidate_all_tags();
        crate::cache::invalidate_post_by_slug(&final_slug).await;
        crate::cache::invalidate_post_stats();

        let all_tags_to_invalidate: std::collections::HashSet<String> = old_tags
            .into_iter()
            .chain(tags_for_invalidation.into_iter())
            .collect();
        for tag_name in &all_tags_to_invalidate {
            crate::cache::invalidate_posts_by_tag(tag_name).await;
        }

        if let Some(ref old) = old_slug {
            if old != &final_slug {
                crate::cache::invalidate_post_by_slug(old).await;
            }
        }

        Ok(CreatePostResponse {
            success: true,
            message: "更新成功".to_string(),
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

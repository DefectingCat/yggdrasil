//! 创建文章接口。
//!
//! 校验标题、内容与 slug，生成唯一 slug 并渲染 Markdown，
//! 在事务中写入 posts 表并同步标签关联，最后失效相关缓存。
//! Dioxus server function，注册在 `/api` 路径下。
//! 仅在 `feature = "server"` 启用的服务端构建中写入数据库。

#![allow(clippy::too_many_arguments)]

use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::{clean_tags, get_current_admin_user, sync_tags};
use super::types::CreatePostResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
#[cfg(feature = "server")]
use crate::db::pool::get_conn;
#[cfg(feature = "server")]
use crate::models::post::PostStatus;

/// 创建一篇新文章。
///
/// 校验标题与内容非空、slug 格式合法；未提供 slug 时从标题自动生成。
/// 首次发布时填充 published_at，并同步标签与缓存失效。
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

    // 标题不能为空。
    if title.trim().is_empty() {
        return Ok(CreatePostResponse {
            success: false,
            message: "标题不能为空".to_string(),
            post_id: None,
            slug: None,
        });
    }

    // 内容不能为空。
    if content_md.trim().is_empty() {
        return Ok(CreatePostResponse {
            success: false,
            message: "内容不能为空".to_string(),
            post_id: None,
            slug: None,
        });
    }

    // 确定基础 slug：用户传入时校验格式，否则由标题生成。
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

        // 渲染 Markdown 为 HTML，并提取目录。
        let rendered = crate::api::markdown::render_markdown_enhanced(&content_md);
        let content_html = rendered.html;
        let toc_html = if rendered.toc_html.is_empty() {
            None::<String>
        } else {
            Some(rendered.toc_html)
        };
        // 未填写摘要时自动从正文提取。
        let summary = summary
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| crate::utils::text::auto_summary(&content_md));
        let post_status = PostStatus::from_str(&status).unwrap_or(PostStatus::Draft);
        let cover_image = cover_image.filter(|s| !s.trim().is_empty());

        // 发布状态的文章设置当前发布时间；草稿则为 None。
        let published_at = if post_status == PostStatus::Published {
            Some(chrono::Utc::now())
        } else {
            None
        };

        let tx = client.transaction().await.map_err(AppError::tx)?;

        // 保证 slug 全局唯一，若冲突则追加数字后缀；在事务内检查避免并发竞态。
        let final_slug = crate::api::slug::ensure_unique_slug(&tx, &base_slug, None).await?;

        // 插入文章记录。
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

        // 清洗标签并在事务中同步 post_tags 关联。
        let tags_cleaned = clean_tags(&tags);
        sync_tags(&tx, post_id, &tags_cleaned).await?;

        tx.commit().await.map_err(AppError::tx)?;

        // 写入成功后失效文章列表、标签与统计缓存。
        crate::cache::invalidate_post_lists();
        crate::cache::invalidate_all_tags();
        crate::cache::invalidate_post_stats();
        // 失效按 slug 缓存，避免之前缓存的 404 继续命中。
        crate::cache::invalidate_post_by_slug(&final_slug).await;

        // 失效该文章涉及的所有标签缓存。
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

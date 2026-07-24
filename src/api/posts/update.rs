//! 更新文章接口。
//!
//! 校验管理员权限与文章归属，重新生成唯一 slug、渲染 Markdown，
//! 在事务中更新 posts 表并同步标签，最后失效相关缓存。
//! Dioxus server function，注册在 `/api` 路径下。
//! 仅在 `feature = "server"` 启用的服务端构建中写入数据库。

#![allow(clippy::too_many_arguments)]

use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::{clean_tags, get_current_admin_user, sync_asset_refs, sync_tags};
use super::types::CreatePostResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
#[cfg(feature = "server")]
use crate::db::pool::get_conn;
#[cfg(feature = "server")]
use crate::models::post::PostStatus;

/// 更新指定文章。
///
/// 校验文章存在且属于当前 admin；处理 slug 变更、发布状态转换、标签同步，
/// 并失效文章详情、列表、标签与统计缓存。
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

        // Markdown 渲染移到阻塞线程池执行。
        let md_for_render = content_md.clone();
        let rendered = tokio::task::spawn_blocking(move || {
            crate::api::markdown::render_markdown_enhanced(&md_for_render)
        })
        .await
        .map_err(|_| AppError::Internal("Markdown 渲染任务失败"))?;
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

        // 重新计算字数与阅读时长，保持与正文同步。
        let word_count = crate::utils::text::count_words(&content_md);
        let reading_time = crate::utils::text::reading_time(word_count);

        let tx = client.transaction().await.map_err(AppError::tx)?;

        // 查询旧 slug，用于后续缓存失效。
        let old_slug: Option<String> = tx
            .query_opt("SELECT slug FROM posts WHERE id = $1", &[&post_id])
            .await
            .map_err(AppError::query)?
            .map(|r| r.get(0));

        // 校验文章存在、未删除且归属当前用户。
        let exists: bool = tx
            .query_opt(
                "SELECT 1 FROM posts WHERE id = $1 AND author_id = $2 AND deleted_at IS NULL",
                &[&post_id, &user.id],
            )
            .await
            .map_err(AppError::query)?
            .is_some();

        if !exists {
            return Ok(CreatePostResponse::err("文章不存在或无权限".to_string()));
        }

        // 确定基础 slug：用户传入时校验格式，否则由标题生成。
        let base_slug = match slug {
            Some(ref s) if !s.trim().is_empty() => {
                let s = s.trim();
                if !crate::api::slug::is_valid_slug(s) {
                    return Ok(CreatePostResponse::err("slug 格式无效".to_string()));
                }
                s.to_string()
            }
            _ => crate::api::slug::slugify(&title),
        };

        // 保证 slug 全局唯一，排除当前文章自身；在事务内检查避免并发竞态。
        let final_slug =
            crate::api::slug::ensure_unique_slug(&tx, &base_slug, Some(post_id)).await?;

        // 获取文章旧标签，用于后续失效标签缓存。
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

        // 获取旧状态与旧发布时间，用于决定是否需要更新 published_at。
        let old_status_row = tx
            .query_opt(
                "SELECT status, published_at FROM posts WHERE id = $1",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        // 发布时：若之前已发布则保留原时间，否则使用当前时间。
        // 非发布时：保留原有 published_at（若为草稿可能为 None）。
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

        // 更新文章主表。
        let updated = tx
            .execute(
                "UPDATE posts SET title = $1, slug = $2, summary = $3, content_md = $4, content_html = $5, toc_html = $6, status = $7, published_at = $8, cover_image = $9, word_count = $10, reading_time = $11, updated_at = NOW()
                 WHERE id = $12",
                &[
                    &title.trim(),
                    &final_slug,
                    &summary,
                    &content_md,
                    &content_html,
                    &toc_html,
                    &post_status.as_str(),
                    &published_at,
                    &cover_image,
                    &(word_count as i32),
                    &(reading_time as i32),
                    &post_id,
                ],
            )
            .await
            .map_err(AppError::tx)?;

        if updated == 0 {
            return Ok(CreatePostResponse::err("文章不存在或无权限".to_string()));
        }

        let tags_cleaned = clean_tags(&tags);
        let tags_for_invalidation = tags_cleaned.clone();

        // 先清除旧标签关联，再重新同步新标签。
        tx.execute("DELETE FROM post_tags WHERE post_id = $1", &[&post_id])
            .await
            .map_err(AppError::tx)?;

        sync_tags(&tx, post_id, &tags_cleaned).await?;

        // 同步素材引用关联（asset_refs）：内部自带 DELETE 再重建。
        sync_asset_refs(&tx, post_id, &content_html, cover_image.as_deref()).await?;

        tx.commit().await.map_err(AppError::tx)?;

        // 失效文章列表、标签、当前 slug 与统计缓存。
        crate::cache::invalidate_post_metadata();
        crate::cache::invalidate_post_by_slug(&final_slug).await;

        // 合并旧标签与新标签，统一失效标签下的文章列表缓存。
        let all_tags_to_invalidate: Vec<String> = old_tags
            .into_iter()
            .chain(tags_for_invalidation.into_iter())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        crate::cache::invalidate_tag_posts_for(&all_tags_to_invalidate).await;

        // 若 slug 发生变更，额外失效旧 slug 缓存。
        if let Some(ref old) = old_slug {
            if old != &final_slug {
                crate::cache::invalidate_post_by_slug(old).await;
                crate::ssr_cache::invalidate_ssr_route(&format!("/post/{old}"));
            }
        }

        // SSR：内容/标签/摘要变化影响详情页与所有列表页。
        crate::ssr_cache::invalidate_ssr_route(&format!("/post/{final_slug}"));
        crate::ssr_cache::invalidate_ssr_all_public();
        crate::ssr_cache::bump_global_generation();

        Ok(CreatePostResponse::ok(
            "更新成功".to_string(),
            post_id,
            final_slug,
        ))
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(CreatePostResponse::err("server only".to_string()))
    }
}

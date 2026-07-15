//! 回收站操作接口：恢复、彻底删除、批量操作与一键清空。
//!
//! 所有接口需要 admin 权限，操作后按影响范围精准失效缓存；
//! 仅在影响集很大（如批量清空）时才回退到全量缓存失效。
//! Dioxus server function，注册在 `/api` 路径下。
//! 仅在 `feature = "server"` 启用的服务端构建中执行数据库操作。

use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::get_current_admin_user;
use super::types::CreatePostResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
#[cfg(feature = "server")]
use crate::api::slug::ensure_unique_slug;
#[cfg(feature = "server")]
use crate::db::pool::get_conn;

/// 批量/清空操作使用精准失效的最大记录数阈值。
/// 超过该阈值时回退到 `invalidate_all_post_caches()`，避免大量串行缓存操作。
#[cfg(feature = "server")]
const PRECISE_INVALIDATION_LIMIT: usize = 50;

/// 恢复一篇已删除的文章（将 deleted_at 置空）。
///
/// 若该文章原始 slug 已被其他未删除文章占用，自动追加数字后缀。
#[server(RestorePost, "/api")]
pub async fn restore_post(post_id: i32) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let mut client = get_conn().await.map_err(AppError::db_conn)?;
        let tx = client.transaction().await.map_err(AppError::tx)?;

        // 在事务内锁定行并读取当前 slug、标签与是否确已删除。
        let row = tx
            .query_opt(
                "SELECT slug FROM posts WHERE id = $1 AND deleted_at IS NOT NULL FOR UPDATE",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        let Some(row) = row else {
            return Ok(CreatePostResponse::err("文章不在回收站".to_string()));
        };

        let current_slug: String = row.get("slug");

        // 恢复时确保 slug 在未删除文章中唯一（自动加后缀）；在事务内检查避免并发竞态。
        let new_slug = ensure_unique_slug(&tx, &current_slug, Some(post_id)).await?;

        let tag_rows = tx
            .query(
                "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;
        let tags: Vec<String> = tag_rows.iter().map(|r| r.get(0)).collect();

        // 置空 deleted_at，并更新 slug（可能已加后缀）。
        let result = tx
            .execute(
                "UPDATE posts SET deleted_at = NULL, slug = $1 WHERE id = $2 AND deleted_at IS NOT NULL",
                &[&new_slug, &post_id],
            )
            .await
            .map_err(AppError::tx)?;

        if result == 0 {
            return Ok(CreatePostResponse::err("文章不在回收站".to_string()));
        }

        tx.commit().await.map_err(AppError::tx)?;

        // 精准失效：列表、标签云、统计、旧 slug 与新 slug、相关标签文章。
        crate::cache::invalidate_post_metadata();
        crate::cache::invalidate_post_by_slug(&current_slug).await;
        crate::cache::invalidate_post_by_slug(&new_slug).await;
        crate::cache::invalidate_tag_posts_for(&tags).await;

        // 递增 SSR 全局世代号（未来就绪基础设施；当前不会使 Dioxus 0.7 SSR 缓存失效）。
        crate::ssr_cache::bump_global_generation();

        Ok(CreatePostResponse::ok(
            "恢复成功".to_string(),
            post_id,
            new_slug,
        ))
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(CreatePostResponse::err("server only".to_string()))
    }
}

/// 彻底删除一篇已删除的文章（物理删除，不可恢复）。
///
/// 注意：仅删除数据库记录，不删除已上传的图片文件。
/// post_tags 关联因外键 ON DELETE CASCADE 自动清理。
#[server(PurgePost, "/api")]
pub async fn purge_post(post_id: i32) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let mut client = get_conn().await.map_err(AppError::db_conn)?;
        let tx = client.transaction().await.map_err(AppError::tx)?;

        // 在事务内锁定行并读取 slug 与标签，避免并发更新导致缓存失效目标过期。
        let slug_row = tx
            .query_opt(
                "SELECT slug FROM posts WHERE id = $1 AND deleted_at IS NOT NULL FOR UPDATE",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        let Some(slug_row) = slug_row else {
            return Ok(CreatePostResponse::err("文章不在回收站".to_string()));
        };
        let slug: String = slug_row.get(0);

        let tag_rows = tx
            .query(
                "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;
        let tags: Vec<String> = tag_rows.iter().map(|r| r.get(0)).collect();

        let result = tx
            .execute(
                "DELETE FROM posts WHERE id = $1 AND deleted_at IS NOT NULL",
                &[&post_id],
            )
            .await
            .map_err(AppError::tx)?;

        if result == 0 {
            return Ok(CreatePostResponse::err("文章不在回收站".to_string()));
        }

        tx.commit().await.map_err(AppError::tx)?;

        // 精准失效相关缓存。
        crate::cache::invalidate_post_metadata();
        crate::cache::invalidate_post_by_slug(&slug).await;
        crate::cache::invalidate_tag_posts_for(&tags).await;

        // 递增 SSR 全局世代号（未来就绪基础设施；当前不会使 Dioxus 0.7 SSR 缓存失效）。
        crate::ssr_cache::bump_global_generation();

        Ok(CreatePostResponse::ok(
            "彻底删除成功".to_string(),
            post_id,
            slug,
        ))
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(CreatePostResponse::err("server only".to_string()))
    }
}

/// 批量恢复文章。
#[server(BatchRestorePosts, "/api")]
pub async fn batch_restore_posts(post_ids: Vec<i32>) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        if post_ids.is_empty() {
            return Ok(CreatePostResponse::ok_msg("无操作".to_string()));
        }

        let mut client = get_conn().await.map_err(AppError::db_conn)?;
        let tx = client.transaction().await.map_err(AppError::tx)?;

        // 记录数较少时使用精准失效；否则回退到全量失效。
        let use_precise = post_ids.len() <= PRECISE_INVALIDATION_LIMIT;

        // 逐条恢复，slug 冲突时自动加后缀；同时收集受影响的 slug 与标签。
        let mut restored = 0u64;
        let mut affected_slugs: Vec<String> = Vec::with_capacity(post_ids.len() * 2);
        let mut affected_tags: std::collections::HashSet<String> = std::collections::HashSet::new();

        for id in &post_ids {
            let row = tx
                .query_opt(
                    "SELECT slug FROM posts WHERE id = $1 AND deleted_at IS NOT NULL FOR UPDATE",
                    &[&id],
                )
                .await
                .map_err(AppError::query)?;
            if let Some(row) = row {
                let current_slug: String = row.get("slug");
                let new_slug = ensure_unique_slug(&tx, &current_slug, Some(*id)).await?;

                if use_precise {
                    let tag_rows = tx
                        .query(
                            "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1",
                            &[&id],
                        )
                        .await
                        .map_err(AppError::query)?;
                    for tag_row in &tag_rows {
                        affected_tags.insert(tag_row.get(0));
                    }
                }

                let n = tx
                    .execute(
                        "UPDATE posts SET deleted_at = NULL, slug = $1 WHERE id = $2 AND deleted_at IS NOT NULL",
                        &[&new_slug, &id],
                    )
                    .await
                    .map_err(AppError::tx)?;
                restored += n;

                if use_precise {
                    affected_slugs.push(current_slug);
                    affected_slugs.push(new_slug);
                }
            }
        }

        tx.commit().await.map_err(AppError::tx)?;

        if use_precise {
            // 精准失效：先去重 slug，再统一失效列表/标签云/统计/标签文章。
            let unique_slugs: std::collections::HashSet<String> =
                affected_slugs.into_iter().collect();
            crate::cache::invalidate_post_metadata();
            for slug in &unique_slugs {
                crate::cache::invalidate_post_by_slug(slug).await;
            }
            crate::cache::invalidate_tag_posts_for(&affected_tags.into_iter().collect::<Vec<_>>())
                .await;

            // 递增 SSR 全局世代号（未来就绪基础设施；当前不会使 Dioxus 0.7 SSR 缓存失效）。
            crate::ssr_cache::bump_global_generation();
        } else {
            // 影响集过大时回退到全量失效，避免大量串行缓存操作。
            crate::cache::invalidate_all_post_caches();
            crate::cache::invalidate_search_results();
            // 递增 SSR 全局世代号（未来就绪基础设施；当前不会使 Dioxus 0.7 SSR 缓存失效）。
            crate::ssr_cache::bump_global_generation();
        }

        Ok(CreatePostResponse::ok_msg(format!("已恢复 {restored} 篇")))
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(CreatePostResponse::err("server only".to_string()))
    }
}

/// 批量彻底删除文章。
#[server(BatchPurgePosts, "/api")]
pub async fn batch_purge_posts(post_ids: Vec<i32>) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        if post_ids.is_empty() {
            return Ok(CreatePostResponse::ok_msg("无操作".to_string()));
        }

        let mut client = get_conn().await.map_err(AppError::db_conn)?;
        let tx = client.transaction().await.map_err(AppError::tx)?;
        let total = post_ids.len() as i64;

        // 记录数较少时锁定行并读取 slug 与标签，使用精准失效；否则回退到全量失效。
        let use_precise = post_ids.len() <= PRECISE_INVALIDATION_LIMIT;
        let (slugs, tags) = if use_precise {
            let mut slugs = Vec::with_capacity(post_ids.len());
            let mut tags_set: std::collections::HashSet<String> = std::collections::HashSet::new();

            for id in &post_ids {
                let slug_row = tx
                    .query_opt(
                        "SELECT slug FROM posts WHERE id = $1 AND deleted_at IS NOT NULL FOR UPDATE",
                        &[&id],
                    )
                    .await
                    .map_err(AppError::query)?;

                if let Some(slug_row) = slug_row {
                    let slug: String = slug_row.get(0);
                    let tag_rows = tx
                        .query(
                            "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1",
                            &[&id],
                        )
                        .await
                        .map_err(AppError::query)?;
                    for tag_row in &tag_rows {
                        tags_set.insert(tag_row.get(0));
                    }
                    slugs.push(slug);
                }
            }

            (slugs, tags_set.into_iter().collect::<Vec<_>>())
        } else {
            (Vec::new(), Vec::new())
        };

        let result = tx
            .execute(
                "DELETE FROM posts WHERE id = ANY($1) AND deleted_at IS NOT NULL",
                &[&post_ids],
            )
            .await
            .map_err(AppError::tx)?;

        tx.commit().await.map_err(AppError::tx)?;

        if use_precise {
            crate::cache::invalidate_post_metadata();
            for slug in &slugs {
                crate::cache::invalidate_post_by_slug(slug).await;
            }
            crate::cache::invalidate_tag_posts_for(&tags).await;

            // 递增 SSR 全局世代号（未来就绪基础设施；当前不会使 Dioxus 0.7 SSR 缓存失效）。
            crate::ssr_cache::bump_global_generation();
        } else {
            // 影响集过大时回退到全量失效，避免大量串行缓存操作。
            crate::cache::invalidate_all_post_caches();
            crate::cache::invalidate_search_results();
            // 递增 SSR 全局世代号（未来就绪基础设施；当前不会使 Dioxus 0.7 SSR 缓存失效）。
            crate::ssr_cache::bump_global_generation();
        }

        Ok(CreatePostResponse::ok_msg(format!(
            "已彻底删除 {result}/{total} 篇"
        )))
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(CreatePostResponse::err("server only".to_string()))
    }
}

/// 清空回收站：彻底删除所有已软删除的文章。
#[server(EmptyTrash, "/api")]
pub async fn empty_trash() -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let mut client = get_conn().await.map_err(AppError::db_conn)?;
        let tx = client.transaction().await.map_err(AppError::tx)?;

        // 在事务内锁定所有待删除行并读取 id/slug，用于后续精准失效；
        // 同时根据数量决定使用精准失效还是回退到全量失效。
        let deleted_rows = tx
            .query(
                "SELECT id, slug FROM posts WHERE deleted_at IS NOT NULL FOR UPDATE",
                &[],
            )
            .await
            .map_err(AppError::query)?;
        let use_precise =
            !deleted_rows.is_empty() && deleted_rows.len() <= PRECISE_INVALIDATION_LIMIT;

        let (slugs, tags) = if use_precise {
            let slugs: Vec<String> = deleted_rows.iter().map(|r| r.get("slug")).collect();
            let ids: Vec<i32> = deleted_rows.iter().map(|r| r.get("id")).collect();

            let tag_rows = tx
                .query(
                    "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = ANY($1)",
                    &[&ids],
                )
                .await
                .map_err(AppError::query)?;
            let tags: Vec<String> = tag_rows.iter().map(|r| r.get(0)).collect();

            (slugs, tags)
        } else {
            (Vec::new(), Vec::new())
        };

        let result = tx
            .execute("DELETE FROM posts WHERE deleted_at IS NOT NULL", &[])
            .await
            .map_err(AppError::tx)?;

        tx.commit().await.map_err(AppError::tx)?;

        if use_precise {
            crate::cache::invalidate_post_metadata();
            for slug in &slugs {
                crate::cache::invalidate_post_by_slug(slug).await;
            }
            crate::cache::invalidate_tag_posts_for(&tags).await;

            // 递增 SSR 全局世代号（未来就绪基础设施；当前不会使 Dioxus 0.7 SSR 缓存失效）。
            crate::ssr_cache::bump_global_generation();
        } else {
            // 影响集过大时回退到全量失效，避免大量串行缓存操作。
            crate::cache::invalidate_all_post_caches();
            crate::cache::invalidate_search_results();
            // 递增 SSR 全局世代号（未来就绪基础设施；当前不会使 Dioxus 0.7 SSR 缓存失效）。
            crate::ssr_cache::bump_global_generation();
        }

        Ok(CreatePostResponse::ok_msg(format!(
            "已清空回收站（{result} 篇）"
        )))
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(CreatePostResponse::err("server only".to_string()))
    }
}

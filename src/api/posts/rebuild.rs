//! 批量重建文章 HTML 与目录。
//!
//! 用于数据迁移或修复：遍历符合条件的文章，将 Markdown 重新渲染为 HTML，
//! 并更新 content_html 与 toc_html 字段。
//! Dioxus server function，注册在 `/api` 路径下。
//! 仅在 `feature = "server"` 启用的服务端构建中执行数据库更新。

use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::get_current_admin_user;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::api::posts::{CreatePostResponse, RebuildResult};
#[cfg(feature = "server")]
use crate::db::pool::get_conn;

/// 单次重建批处理数量上限。
#[cfg(feature = "server")]
const REBUILD_BATCH_LIMIT: i64 = 500;
/// 返回给前端展示的最大错误条数。
#[cfg(feature = "server")]
const MAX_DISPLAY_ERRORS: usize = 5;

/// 批量重建文章 content_html 与 toc_html。
///
/// 当 `rebuild_all` 为 true 时重建所有未删除文章；否则仅重建 content_html 为空的文章。
/// 单批最多处理 500 条，渲染异常或写入失败会被捕获并汇总。
#[server(RebuildContentHtml, "/api")]
pub async fn rebuild_content_html(rebuild_all: bool) -> Result<RebuildResult, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let mut client = get_conn().await.map_err(AppError::db_conn)?;

        // 根据参数构造 WHERE 条件，限制单次处理数量。
        // SELECT 在事务内并加 FOR UPDATE，锁住待处理行直到 UPDATE 完成，避免
        // 并发编辑造成非可重复读（review 发现）：事务外读到的 content_md 可能在
        // UPDATE 前被并发请求修改，导致用旧内容覆盖新内容。
        let query = if rebuild_all {
            format!(
                "SELECT id, content_md FROM posts WHERE deleted_at IS NULL ORDER BY id LIMIT {REBUILD_BATCH_LIMIT} FOR UPDATE"
            )
        } else {
            format!(
                "SELECT id, content_md FROM posts WHERE deleted_at IS NULL AND content_html IS NULL ORDER BY id LIMIT {REBUILD_BATCH_LIMIT} FOR UPDATE"
            )
        };

        let mut rebuilt: u64 = 0;
        let mut failed: u64 = 0;
        let mut errors: Vec<String> = Vec::new();

        // 整批 SELECT + UPDATE 纳入单事务：中途断连或写入失败整批回滚，避免产生
        // 「部分文章已重建」的中间态（M5）；FOR UPDATE 锁住的行随事务结束释放。
        let tx = client.transaction().await.map_err(AppError::query)?;

        let rows = tx.query(&query, &[]).await.map_err(AppError::query)?;

        for row in &rows {
            let id: i32 = row.get(0);
            let content_md: String = row.get(1);

            // Markdown 渲染在阻塞线程池执行；spawn_blocking 的 JoinError 自动捕获 panic，
            // 替代原先的 catch_unwind。
            let md_for_render = content_md.clone();
            let rendered = match tokio::task::spawn_blocking(move || {
                crate::api::markdown::render_markdown_enhanced(&md_for_render)
            })
            .await
            {
                Ok(r) => r,
                Err(_) => {
                    failed += 1;
                    if errors.len() < MAX_DISPLAY_ERRORS {
                        errors.push(format!("文章 #{id}: 渲染异常"));
                    }
                    continue;
                }
            };

            let toc_html = if rendered.toc_html.is_empty() {
                None::<String>
            } else {
                Some(rendered.toc_html)
            };

            let word_count = crate::utils::text::count_words(&content_md);
            let reading_time = crate::utils::text::reading_time(word_count);

            match tx
                .execute(
                    "UPDATE posts SET content_html = $1, toc_html = $2, word_count = $3, reading_time = $4 WHERE id = $5",
                    &[
                        &rendered.html,
                        &toc_html,
                        &(word_count as i32),
                        &(reading_time as i32),
                        &id,
                    ],
                )
                .await
            {
                Ok(_) => {
                    rebuilt += 1;
                }
                Err(e) => {
                    // 事务内任一写入失败会使事务进入 abort 状态，后续写入都会失败；
                    // 此时整批回滚，保证不产生中间态。
                    failed += 1;
                    if errors.len() < MAX_DISPLAY_ERRORS {
                        errors.push(format!("文章 #{id}: DB 写入失败（整批将回滚）"));
                    }
                    tracing::error!("rebuild UPDATE 失败，整批回滚: {:?}", e);
                    tx.rollback().await.ok();
                    return Ok(RebuildResult {
                        rebuilt: 0,
                        failed,
                        errors,
                    });
                }
            }
        }

        tx.commit().await.map_err(AppError::query)?;

        // 重建会修改 word_count / reading_time 等列表项字段，批量影响列表、标签云、
        // 标签文章及单篇缓存；这里使用全量失效作为务实的回退策略。
        if rebuilt > 0 {
            crate::cache::invalidate_all_post_caches();
            crate::cache::invalidate_search_results();
            // 递增 SSR 全局世代号（未来就绪基础设施；当前不会使 Dioxus 0.7 SSR 缓存失效）。
            crate::ssr_cache::bump_global_generation();
        }

        Ok(RebuildResult {
            rebuilt,
            failed,
            errors,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(RebuildResult {
            rebuilt: 0,
            failed: 0,
            errors: vec![],
        })
    }
}

/// 重新渲染指定文章的 content_html 与 toc_html。
///
/// 从数据库读取 content_md（`FOR UPDATE` 锁行，避免并发编辑产生非可重复读），
/// 在阻塞线程池渲染 HTML，并更新 content_html / toc_html / word_count / reading_time。
/// 仅 admin 可调用；成功后按影响范围失效文章列表、slug 单篇与搜索缓存。
#[server(RebuildPostContentHtml, "/api")]
pub async fn rebuild_post_content_html(
    post_id: i32,
) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let mut client = get_conn().await.map_err(AppError::db_conn)?;
        let tx = client.transaction().await.map_err(AppError::tx)?;

        // 在事务内锁行并读取 content_md / slug，避免并发更新导致用旧内容覆盖新内容
        // 或缓存失效目标过期（与 delete_post 一致的事务策略）。
        let row = tx
            .query_opt(
                "SELECT content_md, slug FROM posts WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        let Some(row) = row else {
            return Ok(CreatePostResponse::err("文章不存在".to_string()));
        };

        let content_md: String = row.get(0);
        let slug: String = row.get(1);

        // Markdown 渲染在阻塞线程池执行（CPU 密集）。
        let md_for_render = content_md.clone();
        let rendered = tokio::task::spawn_blocking(move || {
            crate::api::markdown::render_markdown_enhanced(&md_for_render)
        })
        .await
        .map_err(|_| AppError::Internal("Markdown 渲染任务失败"))?;

        let toc_html = if rendered.toc_html.is_empty() {
            None::<String>
        } else {
            Some(rendered.toc_html)
        };

        let word_count = crate::utils::text::count_words(&content_md);
        let reading_time = crate::utils::text::reading_time(word_count);

        tx.execute(
            "UPDATE posts SET content_html = $1, toc_html = $2, word_count = $3, reading_time = $4 WHERE id = $5",
            &[
                &rendered.html,
                &toc_html,
                &(word_count as i32),
                &(reading_time as i32),
                &post_id,
            ],
        )
        .await
        .map_err(AppError::query)?;

        tx.commit().await.map_err(AppError::tx)?;

        // 重建修改了 word_count / reading_time 等列表项字段以及单篇 content_html，
        // 按影响范围精准失效（标签与统计不受影响）。
        crate::cache::invalidate_post_lists();
        crate::cache::invalidate_search_results();
        crate::cache::invalidate_post_by_slug(&slug).await;
        // 递增 SSR 全局世代号（未来就绪基础设施；当前不会使 Dioxus 0.7 SSR 缓存失效）。
        crate::ssr_cache::bump_global_generation();

        Ok(CreatePostResponse::ok("重建成功".to_string(), post_id, slug))
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(CreatePostResponse::err("server only".to_string()))
    }
}

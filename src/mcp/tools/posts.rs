//! MCP 写作用域工具：文章 CRUD。
//!
//! 镜像 `src/api/posts/{create,update,trash,delete}.rs` 的 server-fn 逻辑，
//! 但用 bearer-token 鉴权（`principal.user_id` 作 author_id），不走 cookie。
//! 每个写操作后执行与 web 后台完全一致的缓存失效（moka + SSR）。
//!
//! 本模块仅 `feature = "server"` 编译；`server.rs` 在最终装配时把 `posts_router`
//! 组合进单一 `ServerHandler`。

#![cfg(feature = "server")]
#![allow(clippy::too_many_arguments)]

use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router, ErrorData as McpError};
use serde::Deserialize;

use crate::cache;
use crate::db::pool::get_conn;
use crate::models::mcp_token::TokenScope;
use crate::models::post::PostStatus;
use crate::ssr_cache;
use super::common::{internal, ok_json, require_scope};

// ---------------------------------------------------------------------------
// 结构体
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 工具
// ---------------------------------------------------------------------------

#[tool_router(router = posts_router, vis = "pub")]
impl crate::mcp::server::YggMcpServer {
    /// 创建一篇新文章（草稿或直接发布）。要求 write 作用域。
    #[tool(description = "创建一篇新文章。渲染 Markdown 为 HTML，同步标签与素材引用。返回 post_id/slug。")]
    async fn create_post(
        &self,
        Parameters(p): Parameters<CreatePostParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let principal = require_scope(&parts, "create_post", TokenScope::Write)?;

        if p.title.trim().is_empty() {
            return Err(McpError::invalid_request("title must not be empty", None));
        }
        if p.content_md.trim().is_empty() {
            return Err(McpError::invalid_request("content_md must not be empty", None));
        }

        // 确定基础 slug。
        let base_slug = match &p.slug {
            Some(s) if !s.trim().is_empty() => {
                let s = s.trim();
                if !crate::api::slug::is_valid_slug(s) {
                    return Err(McpError::invalid_request(
                        "slug 格式无效，只能包含字母、数字、连字符和下划线",
                        None,
                    ));
                }
                s.to_string()
            }
            _ => crate::api::slug::slugify(&p.title),
        };

        // Markdown 渲染是 CPU 密集任务。
        let md = p.content_md.clone();
        let rendered = tokio::task::spawn_blocking(move || {
            crate::api::markdown::render_markdown_enhanced(&md)
        })
        .await
        .map_err(|e| internal(e, "markdown render"))?;
        let content_html = rendered.html;
        let toc_html = if rendered.toc_html.is_empty() {
            None
        } else {
            Some(rendered.toc_html)
        };
        let summary = p
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| crate::utils::text::auto_summary(&p.content_md));
        let post_status = PostStatus::from_str(&p.status).unwrap_or(PostStatus::Draft);
        let cover_image = p
            .cover_image
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let word_count = crate::utils::text::count_words(&p.content_md);
        let reading_time = crate::utils::text::reading_time(word_count);
        let published_at = if post_status == PostStatus::Published {
            Some(chrono::Utc::now())
        } else {
            None
        };

        let mut client = get_conn().await.map_err(|e| internal(e, "db connection"))?;
        let tx = client
            .transaction()
            .await
            .map_err(|e| internal(e, "begin txn"))?;

        let final_slug =
            crate::api::slug::ensure_unique_slug(&tx, &base_slug, None)
                .await
                .map_err(|e| internal(e, "ensure_unique_slug"))?;

        let row = tx
            .query_one(
                "INSERT INTO posts (author_id, title, slug, summary, content_md, content_html, toc_html, status, published_at, cover_image, word_count, reading_time)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                 RETURNING id",
                &[
                    &principal.user_id,
                    &p.title.trim(),
                    &final_slug,
                    &summary,
                    &p.content_md,
                    &content_html,
                    &toc_html,
                    &post_status.as_str(),
                    &published_at,
                    &cover_image,
                    &(word_count as i32),
                    &(reading_time as i32),
                ],
            )
            .await
            .map_err(|e| internal(e, "insert post"))?;
        let post_id: i32 = row.get(0);

        let tags_cleaned = crate::api::posts::helpers::clean_tags(&p.tags);
        crate::api::posts::helpers::sync_tags(&tx, post_id, &tags_cleaned)
            .await
            .map_err(|_| internal("tag sync", "sync_tags"))?;
        crate::api::posts::helpers::sync_asset_refs(&tx, post_id, &content_html, cover_image.as_deref())
            .await
            .map_err(|_| internal("asset_refs sync", "sync_asset_refs"))?;

        tx.commit().await.map_err(|e| internal(e, "commit"))?;

        // 与 web 后台一致的缓存失效。
        cache::invalidate_post_metadata();
        cache::invalidate_post_by_slug(&final_slug).await;
        cache::invalidate_tag_posts_for(&tags_cleaned).await;
        ssr_cache::invalidate_ssr_all_public();
        ssr_cache::bump_global_generation();

        ok_json(PostResult {
            success: true,
            message: "创建成功".into(),
            post_id: Some(post_id),
            slug: Some(final_slug),
        })
    }

    /// 更新指定文章（重新渲染 Markdown、同步标签与素材引用）。要求 write 作用域。
    /// 仅文章原作者可更新。
    #[tool(description = "更新一篇已有文章。重新渲染 Markdown，同步标签。仅文章原作者可更新。")]
    async fn update_post(
        &self,
        Parameters(p): Parameters<UpdatePostParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let principal = require_scope(&parts, "update_post", TokenScope::Write)?;

        if p.title.trim().is_empty() {
            return Err(McpError::invalid_request("title must not be empty", None));
        }
        if p.content_md.trim().is_empty() {
            return Err(McpError::invalid_request("content_md must not be empty", None));
        }

        let md = p.content_md.clone();
        let rendered = tokio::task::spawn_blocking(move || {
            crate::api::markdown::render_markdown_enhanced(&md)
        })
        .await
        .map_err(|e| internal(e, "markdown render"))?;
        let content_html = rendered.html;
        let toc_html = if rendered.toc_html.is_empty() {
            None
        } else {
            Some(rendered.toc_html)
        };
        let summary = p
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| crate::utils::text::auto_summary(&p.content_md));
        let post_status = PostStatus::from_str(&p.status).unwrap_or(PostStatus::Draft);
        let cover_image = p
            .cover_image
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let word_count = crate::utils::text::count_words(&p.content_md);
        let reading_time = crate::utils::text::reading_time(word_count);

        let mut client = get_conn().await.map_err(|e| internal(e, "db connection"))?;
        let tx = client
            .transaction()
            .await
            .map_err(|e| internal(e, "begin txn"))?;

        // 查旧 slug（用于缓存失效）。
        let old_slug: Option<String> = tx
            .query_opt("SELECT slug FROM posts WHERE id = $1", &[&p.post_id])
            .await
            .map_err(|e| internal(e, "select old slug"))?
            .map(|r| r.get(0));

        // 校验存在、未删除、归属当前用户。
        let exists: bool = tx
            .query_opt(
                "SELECT 1 FROM posts WHERE id = $1 AND author_id = $2 AND deleted_at IS NULL",
                &[&p.post_id, &principal.user_id],
            )
            .await
            .map_err(|e| internal(e, "check ownership"))?
            .is_some();
        if !exists {
            return Err(McpError::invalid_request(
                "文章不存在或无权限",
                None,
            ));
        }

        // 确定基础 slug。
        let base_slug = match &p.slug {
            Some(s) if !s.trim().is_empty() => {
                let s = s.trim();
                if !crate::api::slug::is_valid_slug(s) {
                    return Err(McpError::invalid_request("slug 格式无效", None));
                }
                s.to_string()
            }
            _ => crate::api::slug::slugify(&p.title),
        };
        let final_slug =
            crate::api::slug::ensure_unique_slug(&tx, &base_slug, Some(p.post_id))
                .await
                .map_err(|e| internal(e, "ensure_unique_slug"))?;

        // 旧标签。
        let old_tags: Vec<String> = {
            let rows = tx
                .query(
                    "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1",
                    &[&p.post_id],
                )
                .await
                .map_err(|e| internal(e, "select old tags"))?;
            rows.iter().map(|r| r.get(0)).collect()
        };

        // 旧状态/发布时间 → 计算新 published_at。
        let old_status_row = tx
            .query_opt(
                "SELECT status, published_at FROM posts WHERE id = $1",
                &[&p.post_id],
            )
            .await
            .map_err(|e| internal(e, "select old status"))?;
        let published_at = if post_status == PostStatus::Published {
            let was_published = old_status_row
                .as_ref()
                .map(|r| {
                    let s: String = r.get(0);
                    s == "published"
                })
                .unwrap_or(false);
            let existing: Option<chrono::DateTime<chrono::Utc>> =
                old_status_row.as_ref().and_then(|r| r.get(1));
            if was_published {
                existing
            } else {
                Some(chrono::Utc::now())
            }
        } else {
            old_status_row.and_then(|r| r.get(1))
        };

        let updated = tx
            .execute(
                "UPDATE posts SET title = $1, slug = $2, summary = $3, content_md = $4, content_html = $5, toc_html = $6, status = $7, published_at = $8, cover_image = $9, word_count = $10, reading_time = $11, updated_at = NOW()
                 WHERE id = $12",
                &[
                    &p.title.trim(),
                    &final_slug,
                    &summary,
                    &p.content_md,
                    &content_html,
                    &toc_html,
                    &post_status.as_str(),
                    &published_at,
                    &cover_image,
                    &(word_count as i32),
                    &(reading_time as i32),
                    &p.post_id,
                ],
            )
            .await
            .map_err(|e| internal(e, "update post"))?;
        if updated == 0 {
            return Err(McpError::invalid_request(
                "文章不存在或无权限",
                None,
            ));
        }

        let tags_cleaned = crate::api::posts::helpers::clean_tags(&p.tags);
        let tags_for_invalidation = tags_cleaned.clone();

        tx.execute("DELETE FROM post_tags WHERE post_id = $1", &[&p.post_id])
            .await
            .map_err(|e| internal(e, "delete old post_tags"))?;
        crate::api::posts::helpers::sync_tags(&tx, p.post_id, &tags_cleaned)
            .await
            .map_err(|_| internal("tag sync", "sync_tags"))?;
        crate::api::posts::helpers::sync_asset_refs(&tx, p.post_id, &content_html, cover_image.as_deref())
            .await
            .map_err(|_| internal("asset_refs sync", "sync_asset_refs"))?;

        tx.commit().await.map_err(|e| internal(e, "commit"))?;

        cache::invalidate_post_metadata();
        cache::invalidate_post_by_slug(&final_slug).await;

        let all_tags: Vec<String> = old_tags
            .into_iter()
            .chain(tags_for_invalidation)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        cache::invalidate_tag_posts_for(&all_tags).await;

        if let Some(old) = &old_slug {
            if old != &final_slug {
                cache::invalidate_post_by_slug(old).await;
                ssr_cache::invalidate_ssr_route(&format!("/post/{old}"));
            }
        }
        ssr_cache::invalidate_ssr_route(&format!("/post/{final_slug}"));
        ssr_cache::invalidate_ssr_all_public();
        ssr_cache::bump_global_generation();

        ok_json(PostResult {
            success: true,
            message: "更新成功".into(),
            post_id: Some(p.post_id),
            slug: Some(final_slug),
        })
    }

    /// 发布指定文章（设置 status=published 与 published_at）。要求 write 作用域。
    #[tool(description = "发布一篇草稿文章。设置 status=published，若首次发布则填充 published_at。")]
    async fn publish_post(
        &self,
        Parameters(p): Parameters<PostIdParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let principal = require_scope(&parts, "publish_post", TokenScope::Write)?;

        let client = get_conn().await.map_err(|e| internal(e, "db connection"))?;

        // 校验存在、未删除、归属当前用户，并取 slug 用于缓存失效。
        let row = client
            .query_opt(
                "SELECT slug FROM posts WHERE id = $1 AND author_id = $2 AND deleted_at IS NULL",
                &[&p.post_id, &principal.user_id],
            )
            .await
            .map_err(|e| internal(e, "select post"))?;
        let slug: String = match row {
            Some(r) => r.get(0),
            None => {
                return Err(McpError::invalid_request(
                    "文章不存在或无权限",
                    None,
                ));
            }
        };

        // M6 修复：发布后文章出现在公开标签列表页，须失效标签缓存（api update.rs:202
        // 会失效，此 MCP 路径此前漏掉 → 标签页新发文陈旧 ≤120s）。
        let tag_rows = client
            .query(
                "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1",
                &[&p.post_id],
            )
            .await
            .map_err(|e| internal(e, "select tags"))?;
        let tags: Vec<String> = tag_rows.iter().map(|r| r.get(0)).collect();

        let result = client
            .execute(
                "UPDATE posts SET status = 'published', \
                 published_at = COALESCE(published_at, NOW()), updated_at = NOW() \
                 WHERE id = $1 AND deleted_at IS NULL",
                &[&p.post_id],
            )
            .await
            .map_err(|e| internal(e, "publish post"))?;
        if result == 0 {
            return Err(McpError::invalid_request("文章不存在", None));
        }

        cache::invalidate_post_metadata();
        cache::invalidate_post_by_slug(&slug).await;
        cache::invalidate_tag_posts_for(&tags).await;
        ssr_cache::invalidate_ssr_route(&format!("/post/{slug}"));
        ssr_cache::invalidate_ssr_all_public();
        ssr_cache::bump_global_generation();

        ok_json(PostResult {
            success: true,
            message: "发布成功".into(),
            post_id: Some(p.post_id),
            slug: Some(slug),
        })
    }

    /// 将指定文章移入回收站（软删除：设置 deleted_at）。要求 write 作用域。
    #[tool(description = "将文章移入回收站（软删除）。可通过恢复操作还原。")]
    async fn trash_post(
        &self,
        Parameters(p): Parameters<PostIdParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let principal = require_scope(&parts, "trash_post", TokenScope::Write)?;

        let mut client = get_conn().await.map_err(|e| internal(e, "db connection"))?;
        let tx = client
            .transaction()
            .await
            .map_err(|e| internal(e, "begin txn"))?;

        let slug_row = tx
            .query_opt(
                "SELECT slug FROM posts WHERE id = $1 AND author_id = $2 AND deleted_at IS NULL FOR UPDATE",
                &[&p.post_id, &principal.user_id],
            )
            .await
            .map_err(|e| internal(e, "select post"))?;
        let Some(slug_row) = slug_row else {
            return Err(McpError::invalid_request("文章不存在", None));
        };
        let slug: String = slug_row.get(0);

        let tag_rows = tx
            .query(
                "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1",
                &[&p.post_id],
            )
            .await
            .map_err(|e| internal(e, "select tags"))?;
        let tags: Vec<String> = tag_rows.iter().map(|r| r.get(0)).collect();

        let result = tx
            .execute(
                "UPDATE posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
                &[&p.post_id],
            )
            .await
            .map_err(|e| internal(e, "soft delete"))?;
        if result == 0 {
            return Err(McpError::invalid_request("文章不存在", None));
        }

        tx.commit().await.map_err(|e| internal(e, "commit"))?;

        cache::invalidate_post_metadata();
        cache::invalidate_post_by_slug(&slug).await;
        cache::invalidate_tag_posts_for(&tags).await;
        ssr_cache::invalidate_ssr_route(&format!("/post/{slug}"));
        ssr_cache::invalidate_ssr_all_public();
        ssr_cache::bump_global_generation();

        ok_json(PostResult {
            success: true,
            message: "已移入回收站".into(),
            post_id: Some(p.post_id),
            slug: Some(slug),
        })
    }

    /// 彻底删除指定文章（物理删除，不可恢复）。要求 write 作用域。
    #[tool(description = "彻底删除文章（物理删除，不可恢复）。post_tags 关联因外键 CASCADE 自动清理。")]
    async fn delete_post(
        &self,
        Parameters(p): Parameters<PostIdParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let principal = require_scope(&parts, "delete_post", TokenScope::Write)?;

        let mut client = get_conn().await.map_err(|e| internal(e, "db connection"))?;
        let tx = client
            .transaction()
            .await
            .map_err(|e| internal(e, "begin txn"))?;

        let slug_row = tx
            .query_opt(
                "SELECT slug FROM posts WHERE id = $1 AND author_id = $2 FOR UPDATE",
                &[&p.post_id, &principal.user_id],
            )
            .await
            .map_err(|e| internal(e, "select post"))?;
        let Some(slug_row) = slug_row else {
            return Err(McpError::invalid_request("文章不存在", None));
        };
        let slug: String = slug_row.get(0);

        let tag_rows = tx
            .query(
                "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1",
                &[&p.post_id],
            )
            .await
            .map_err(|e| internal(e, "select tags"))?;
        let tags: Vec<String> = tag_rows.iter().map(|r| r.get(0)).collect();

        let result = tx
            .execute("DELETE FROM posts WHERE id = $1", &[&p.post_id])
            .await
            .map_err(|e| internal(e, "hard delete"))?;
        if result == 0 {
            return Err(McpError::invalid_request("文章不存在", None));
        }

        tx.commit().await.map_err(|e| internal(e, "commit"))?;

        cache::invalidate_post_metadata();
        cache::invalidate_post_by_slug(&slug).await;
        cache::invalidate_tag_posts_for(&tags).await;
        ssr_cache::invalidate_ssr_route(&format!("/post/{slug}"));
        ssr_cache::invalidate_ssr_all_public();
        ssr_cache::bump_global_generation();

        ok_json(PostResult {
            success: true,
            message: "已彻底删除".into(),
            post_id: Some(p.post_id),
            slug: Some(slug),
        })
    }
}

// ---------------------------------------------------------------------------
// 参数与输出结构
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreatePostParams {
    /// 文章标题（必填，非空）。
    pub title: String,
    /// Markdown 正文（必填，非空）。
    pub content_md: String,
    /// 摘要；未提供时自动从正文提取。
    #[serde(default)]
    pub summary: Option<String>,
    /// URL slug；未提供时从标题自动生成。
    #[serde(default)]
    pub slug: Option<String>,
    /// 标签列表。
    #[serde(default)]
    pub tags: Vec<String>,
    /// 状态：`draft`（默认）或 `published`。
    #[serde(default = "default_status")]
    pub status: String,
    /// 封面图 URL。
    #[serde(default)]
    pub cover_image: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdatePostParams {
    /// 要更新的文章 id。
    pub post_id: i32,
    /// 新标题。
    pub title: String,
    /// 新 Markdown 正文。
    pub content_md: String,
    /// 新摘要。
    #[serde(default)]
    pub summary: Option<String>,
    /// 新 slug。
    #[serde(default)]
    pub slug: Option<String>,
    /// 新标签列表（完全替换旧标签）。
    #[serde(default)]
    pub tags: Vec<String>,
    /// 新状态。
    #[serde(default = "default_status")]
    pub status: String,
    /// 新封面图 URL。
    #[serde(default)]
    pub cover_image: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PostIdParams {
    /// 文章 id。
    pub post_id: i32,
}

#[derive(Debug, serde::Serialize)]
struct PostResult {
    success: bool,
    message: String,
    post_id: Option<i32>,
    slug: Option<String>,
}

fn default_status() -> String {
    "draft".to_string()
}


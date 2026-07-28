//! MCP 写作用域工具：评论审核管理。
//!
//! 镜像 `src/api/comments/{list,update}.rs` 的 server-fn 逻辑，
//! 但用 bearer-token 鉴权，不走 cookie。
//! 状态变更后执行与 web 后台一致的评论缓存失效。
//!
//! 本模块仅 `feature = "server"` 编译。

#![cfg(feature = "server")]

use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router, ErrorData as McpError};
use serde::Deserialize;

use crate::cache;
use crate::db::pool::get_conn;
use crate::models::mcp_token::TokenScope;
use super::common::{internal, ok_json, require_scope};

#[tool_router(router = comments_router, vis = "pub")]
impl crate::mcp::server::YggMcpServer {
    /// 列出评论（分页，可按状态筛选）。要求 write 作用域。
    #[tool(description = "列出全部评论（分页，每页 20 条）。可按状态筛选：pending/approved/spam/trash。")]
    async fn list_comments(
        &self,
        Parameters(p): Parameters<ListCommentsParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let _principal = require_scope(&parts, "list_comments", TokenScope::Write)?;

        let page = p.page.unwrap_or(1).max(1);
        let per_page: i64 = 20;
        let offset: i64 = (page as i64 - 1) * per_page;

        let client = get_conn().await.map_err(|e| internal(e, "db connection"))?;

        let (total, rows) = match p.status.as_deref() {
            Some(s) if !s.is_empty() => {
                let total: i64 = client
                    .query_one(
                        "SELECT COUNT(*) FROM comments WHERE status = $1 AND deleted_at IS NULL",
                        &[&s],
                    )
                    .await
                    .map_err(|e| internal(e, "count comments"))?
                    .get(0);
                let rows = client
                    .query(
                        "SELECT c.id, c.post_id, c.parent_id, c.depth, c.author_name, \
                                c.author_email, c.author_url, c.content_md, c.status, c.created_at, \
                                p.title as post_title, p.slug as post_slug \
                         FROM comments c JOIN posts p ON c.post_id = p.id \
                         WHERE c.status = $1 AND c.deleted_at IS NULL \
                         ORDER BY c.created_at DESC LIMIT $2 OFFSET $3",
                        &[&s, &per_page, &offset],
                    )
                    .await
                    .map_err(|e| internal(e, "query comments"))?;
                (total, rows)
            }
            _ => {
                let total: i64 = client
                    .query_one(
                        "SELECT COUNT(*) FROM comments WHERE deleted_at IS NULL",
                        &[],
                    )
                    .await
                    .map_err(|e| internal(e, "count comments"))?
                    .get(0);
                let rows = client
                    .query(
                        "SELECT c.id, c.post_id, c.parent_id, c.depth, c.author_name, \
                                c.author_email, c.author_url, c.content_md, c.status, c.created_at, \
                                p.title as post_title, p.slug as post_slug \
                         FROM comments c JOIN posts p ON c.post_id = p.id \
                         WHERE c.deleted_at IS NULL \
                         ORDER BY c.created_at DESC LIMIT $1 OFFSET $2",
                        &[&per_page, &offset],
                    )
                    .await
                    .map_err(|e| internal(e, "query comments"))?;
                (total, rows)
            }
        };

        let comments: Vec<CommentItem> = rows
            .iter()
            .map(|r| CommentItem {
                id: r.get("id"),
                post_id: r.get("post_id"),
                post_title: r.get("post_title"),
                post_slug: r.get("post_slug"),
                parent_id: r.get("parent_id"),
                depth: r.get("depth"),
                author_name: r.get("author_name"),
                author_url: r.get("author_url"),
                content_md: r.get("content_md"),
                status: r.get("status"),
                created_at: r.get::<_, chrono::DateTime<chrono::Utc>>("created_at").to_rfc3339(),
            })
            .collect();

        ok_json(CommentsList {
            comments,
            total,
            page,
            per_page,
        })
    }

    /// 通过指定评论（同时递归通过所有 pending 祖先评论）。要求 write 作用域。
    #[tool(description = "通过指定评论。同时递归通过所有 pending 的祖先评论，确保嵌套链可见。")]
    async fn approve_comment(
        &self,
        Parameters(p): Parameters<CommentIdParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let _principal = require_scope(&parts, "approve_comment", TokenScope::Write)?;

        let client = get_conn().await.map_err(|e| internal(e, "db connection"))?;

        let row = client
            .query_opt(
                "SELECT post_id FROM comments WHERE id = $1 AND deleted_at IS NULL",
                &[&p.comment_id],
            )
            .await
            .map_err(|e| internal(e, "select comment"))?;
        let post_id: i32 = match row {
            Some(r) => r.get(0),
            None => {
                return Err(McpError::invalid_request("评论不存在", None));
            }
        };

        // 通过目标评论。
        client
            .execute(
                "UPDATE comments SET status = 'approved', approved_at = NOW() WHERE id = $1",
                &[&p.comment_id],
            )
            .await
            .map_err(|e| internal(e, "approve comment"))?;

        // 递归向上查找所有 pending 父评论并同步通过。
        client
            .execute(
                "WITH RECURSIVE ancestors AS ( \
                     SELECT parent_id FROM comments WHERE id = $1 \
                     UNION ALL \
                     SELECT c.parent_id FROM comments c JOIN ancestors a ON c.id = a.parent_id WHERE a.parent_id IS NOT NULL \
                 ) \
                 UPDATE comments SET status = 'approved', approved_at = NOW() \
                 WHERE id IN (SELECT parent_id FROM ancestors WHERE parent_id IS NOT NULL) AND status = 'pending'",
                &[&p.comment_id],
            )
            .await
            .map_err(|e| internal(e, "approve ancestors"))?;

        cache::invalidate_comments_by_post(post_id).await;
        cache::invalidate_pending_count().await;

        ok_json(CommentResult {
            success: true,
            message: "已通过".into(),
        })
    }

    /// 删除指定评论（软删除：设置 deleted_at 与 status=trash）。要求 write 作用域。
    #[tool(description = "删除指定评论（移入回收站，软删除）。")]
    async fn delete_comment(
        &self,
        Parameters(p): Parameters<CommentIdParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let _principal = require_scope(&parts, "delete_comment", TokenScope::Write)?;

        let client = get_conn().await.map_err(|e| internal(e, "db connection"))?;

        let row = client
            .query_opt(
                "SELECT post_id FROM comments WHERE id = $1 AND deleted_at IS NULL",
                &[&p.comment_id],
            )
            .await
            .map_err(|e| internal(e, "select comment"))?;
        if let Some(r) = row {
            let post_id: i32 = r.get(0);
            client
                .execute(
                    "UPDATE comments SET status = 'trash', deleted_at = NOW() WHERE id = $1",
                    &[&p.comment_id],
                )
                .await
                .map_err(|e| internal(e, "trash comment"))?;
            cache::invalidate_comments_by_post(post_id).await;
            cache::invalidate_pending_count().await;
        }

        ok_json(CommentResult {
            success: true,
            message: "已删除".into(),
        })
    }

    /// 设置评论状态（approved/spam/trash）。要求 write 作用域。
    #[tool(description = "设置评论审核状态。status 可选 approved/spam/trash。trash 会软删除。")]
    async fn set_comment_status(
        &self,
        Parameters(p): Parameters<SetCommentStatusParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let _principal = require_scope(&parts, "set_comment_status", TokenScope::Write)?;

        let normalized = p.status.trim().to_lowercase();
        if !matches!(normalized.as_str(), "approved" | "spam" | "trash") {
            return Err(McpError::invalid_request(
                "status must be one of: approved, spam, trash",
                None,
            ));
        }

        let client = get_conn().await.map_err(|e| internal(e, "db connection"))?;

        let row = client
            .query_opt(
                "SELECT post_id, status FROM comments WHERE id = $1 AND deleted_at IS NULL",
                &[&p.comment_id],
            )
            .await
            .map_err(|e| internal(e, "select comment"))?;
        match row {
            Some(r) => {
                let post_id: i32 = r.get(0);
                let old_status: String = r.get(1);

                match normalized.as_str() {
                    "approved" => {
                        client
                            .execute(
                                "UPDATE comments SET status = 'approved', approved_at = NOW() WHERE id = $1",
                                &[&p.comment_id],
                            )
                            .await
                            .map_err(|e| internal(e, "set approved"))?;
                    }
                    "spam" => {
                        client
                            .execute(
                                "UPDATE comments SET status = 'spam' WHERE id = $1 AND deleted_at IS NULL",
                                &[&p.comment_id],
                            )
                            .await
                            .map_err(|e| internal(e, "set spam"))?;
                    }
                    "trash" => {
                        client
                            .execute(
                                "UPDATE comments SET status = 'trash', deleted_at = NOW() WHERE id = $1",
                                &[&p.comment_id],
                            )
                            .await
                            .map_err(|e| internal(e, "set trash"))?;
                    }
                    _ => unreachable!("validated above"),
                }

                // 与 web 后台一致：仅当旧状态是 approved 时需失效评论列表缓存。
                if old_status == "approved" || normalized == "approved" {
                    cache::invalidate_comments_by_post(post_id).await;
                }
                cache::invalidate_pending_count().await;
            }
            None => {
                return Err(McpError::invalid_request("评论不存在", None));
            }
        }

        ok_json(CommentResult {
            success: true,
            message: format!("状态已设为 {normalized}"),
        })
    }
}

// ---------------------------------------------------------------------------
// 参数与输出结构
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListCommentsParams {
    /// 页码（从 1 开始，默认 1）。
    #[serde(default)]
    pub page: Option<i32>,
    /// 按状态筛选：pending / approved / spam / trash。不传则返回全部。
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CommentIdParams {
    /// 评论 id。
    pub comment_id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetCommentStatusParams {
    /// 评论 id。
    pub comment_id: i64,
    /// 目标状态：approved / spam / trash。
    pub status: String,
}

#[derive(Debug, serde::Serialize)]
struct CommentItem {
    id: i64,
    post_id: i32,
    post_title: String,
    post_slug: String,
    parent_id: Option<i64>,
    depth: i32,
    author_name: String,
    author_url: Option<String>,
    content_md: String,
    status: String,
    created_at: String,
}

#[derive(Debug, serde::Serialize)]
struct CommentsList {
    comments: Vec<CommentItem>,
    total: i64,
    page: i32,
    per_page: i64,
}

#[derive(Debug, serde::Serialize)]
struct CommentResult {
    success: bool,
    message: String,
}


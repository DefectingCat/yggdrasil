//! 发表评论接口。
//!
//! 校验作者信息、父评论与目标文章，生成内容哈希防止重复提交，
//! 新评论默认进入 pending 状态等待审核。
//! Dioxus server function，注册在 `/api` 路径下。
//! 仅在 `feature = "server"` 启用的服务端构建中写入数据库。

use crate::api::comments::types::*;
use dioxus::prelude::*;

/// 创建一条新评论。
///
/// 对作者昵称、邮箱、网址与内容进行基础校验；
/// 若目标文章未发布或父评论未通过审核，则拒绝提交；
/// 成功后将评论置为 pending，并清空相关缓存。
#[server(CreateComment, "/api")]
pub async fn create_comment(
    post_id: i32,
    parent_id: Option<i64>,
    author_name: String,
    author_email: String,
    author_url: Option<String>,
    content_md: String,
) -> Result<CommentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::api::comments::helpers::{
            compute_content_hash, validate_comment_content, validate_comment_email,
            validate_comment_name, validate_comment_url,
        };
        use crate::api::error::AppError;
        use crate::cache;
        use crate::db::pool::get_conn;

        // 从 FullstackContext 获取客户端 IP，并进行评论频率限流。
        if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
            let parts = ctx.parts_mut();
            let ip = crate::api::rate_limit::get_client_ip(&parts.headers);
            if let Err(msg) = crate::api::rate_limit::check_comment_limit(&ip) {
                return Ok(CommentResponse {
                    success: false,
                    message: msg,
                    error_code: Some("rate_limited".into()),
                    comment_id: None,
                    avatar_url: None,
                    depth: None,
                });
            }
        }

        // 依次校验昵称、邮箱、网址与评论内容。
        if let Err(e) = validate_comment_name(&author_name) {
            return Ok(CommentResponse {
                success: false,
                message: e,
                error_code: Some("invalid_input".into()),
                comment_id: None,
                avatar_url: None,
                depth: None,
            });
        }
        if let Err(e) = validate_comment_email(&author_email) {
            return Ok(CommentResponse {
                success: false,
                message: e,
                error_code: Some("invalid_input".into()),
                comment_id: None,
                avatar_url: None,
                depth: None,
            });
        }
        if let Some(ref url) = author_url {
            if let Err(e) = validate_comment_url(url) {
                return Ok(CommentResponse {
                    success: false,
                    message: e,
                    error_code: Some("invalid_input".into()),
                    comment_id: None,
                    avatar_url: None,
                    depth: None,
                });
            }
        }
        if let Err(e) = validate_comment_content(&content_md) {
            return Ok(CommentResponse {
                success: false,
                message: e,
                error_code: Some("invalid_input".into()),
                comment_id: None,
                avatar_url: None,
                depth: None,
            });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        // 确认目标文章存在且处于已发布状态。
        let post_row = client
            .query_opt(
                "SELECT status, deleted_at FROM posts WHERE id = $1",
                &[&post_id],
            )
            .await
            .map_err(AppError::query)?;

        match post_row {
            None => {
                return Ok(CommentResponse {
                    success: false,
                    message: "文章不存在".to_string(),
                    error_code: Some("post_not_found".into()),
                    comment_id: None,
                    avatar_url: None,
                    depth: None,
                });
            }
            Some(row) => {
                let status: String = row.get("status");
                let deleted_at: Option<chrono::DateTime<chrono::Utc>> = row.get("deleted_at");
                if status != "published" || deleted_at.is_some() {
                    return Ok(CommentResponse {
                        success: false,
                        message: "文章不存在".to_string(),
                        error_code: Some("post_not_found".into()),
                        comment_id: None,
                        avatar_url: None,
                        depth: None,
                    });
                }
            }
        }

        // 若存在父评论，校验其归属文章与审核状态，并计算当前评论的嵌套深度。
        let mut depth: i32 = 0;
        if let Some(pid) = parent_id {
            let parent_row = client
                .query_opt(
                    "SELECT post_id, status, depth FROM comments WHERE id = $1 AND deleted_at IS NULL",
                    &[&pid],
                )
                .await
                .map_err(AppError::query)?;

            match parent_row {
                None => {
                    return Ok(CommentResponse {
                        success: false,
                        message: "父评论不存在".to_string(),
                        error_code: Some("parent_not_found".into()),
                        comment_id: None,
                        avatar_url: None,
                        depth: None,
                    });
                }
                Some(row) => {
                    let parent_post_id: i32 = row.get("post_id");
                    let parent_status: String = row.get("status");
                    let parent_depth: i32 = row.get("depth");

                    if parent_post_id != post_id {
                        return Ok(CommentResponse {
                            success: false,
                            message: "父评论不存在".to_string(),
                            error_code: Some("parent_not_found".into()),
                            comment_id: None,
                            avatar_url: None,
                            depth: None,
                        });
                    }
                    if parent_status != "approved" {
                        return Ok(CommentResponse {
                            success: false,
                            message: "父评论未通过审核".to_string(),
                            error_code: Some("parent_not_approved".into()),
                            comment_id: None,
                            avatar_url: None,
                            depth: None,
                        });
                    }

                    depth = parent_depth + 1;
                    if depth > 20 {
                        return Ok(CommentResponse {
                            success: false,
                            message: "评论嵌套层级过深".to_string(),
                            error_code: Some("too_deep".into()),
                            comment_id: None,
                            avatar_url: None,
                            depth: None,
                        });
                    }
                }
            }
        }

        // 基于文章、父评论、作者与内容计算哈希，防止短时间重复提交。
        let content_hash = compute_content_hash(post_id, parent_id, &author_name, &content_md);

        let dup: Option<i64> = client
            .query_opt(
                "SELECT id FROM comments WHERE post_id = $1 AND content_hash = $2 AND created_at > NOW() - INTERVAL '5 minutes'",
                &[&post_id, &content_hash],
            )
            .await
            .map_err(AppError::query)?
            .map(|r| r.get(0));

        if dup.is_some() {
            return Ok(CommentResponse {
                success: false,
                message: "请勿重复提交".to_string(),
                error_code: Some("duplicate".into()),
                comment_id: None,
                avatar_url: None,
                depth: None,
            });
        }

        // 将 Markdown 渲染为 HTML，并通过 sanitizer 过滤危险标签。
        let content_html = crate::api::comments::markdown::render_comment_markdown(&content_md);

        // 获取客户端 IP 与 User-Agent，用于反垃圾与审计。
        let ip_address = if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
            let parts = ctx.parts_mut();
            Some(crate::api::rate_limit::get_client_ip(&parts.headers))
        } else {
            None
        };

        let user_agent = if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
            let parts = ctx.parts_mut();
            parts
                .headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        } else {
            None
        };

        // 插入评论，默认状态为 pending，等待管理员审核。
        let row = client
            .query_one(
                "INSERT INTO comments \
                 (post_id, parent_id, depth, author_name, author_email, author_url, \
                  content_md, content_html, content_hash, status, ip_address, user_agent) \
                  VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', $10, $11) \
                  RETURNING id",
                &[
                    &post_id,
                    &parent_id,
                    &depth,
                    &author_name.trim(),
                    &author_email.trim(),
                    &author_url
                        .as_ref()
                        .map(|u| u.trim())
                        .filter(|u| !u.is_empty()),
                    &content_md,
                    &content_html,
                    &content_hash,
                    &ip_address,
                    &user_agent,
                ],
            )
            .await
            .map_err(AppError::query)?;

        let comment_id: i64 = row.get(0);

        // 根据邮箱生成 Gravatar 头像链接。
        let avatar_url = crate::api::comments::helpers::gravatar_url(&author_email);

        // 新评论可能影响文章评论列表与计数，清空相关缓存。
        cache::invalidate_comments_by_post(post_id).await;
        cache::invalidate_comment_count(post_id).await;

        Ok(CommentResponse {
            success: true,
            message: "评论已提交，等待审核".to_string(),
            error_code: None,
            comment_id: Some(comment_id),
            avatar_url: Some(avatar_url),
            depth: Some(depth),
        })
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

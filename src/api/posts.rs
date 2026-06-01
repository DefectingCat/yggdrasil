#![allow(clippy::unused_unit, deprecated, unused_imports)]

use dioxus::prelude::*;

use crate::db::pool::DB_POOL;
use crate::models::post::{Post, PostStatus, PostStats, Tag};
use crate::models::user::{User, UserRole};

// ============================================================================
// Server-side helpers (only compiled when server feature is enabled)
// ============================================================================

#[cfg(feature = "server")]
fn parse_session_token(cookie_header: &str) -> Option<&str> {
    cookie_header
        .split(';')
        .map(|s| s.trim())
        .find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let name = parts.next()?.trim();
            let value = parts.next()?.trim();
            if name == "session" {
                Some(value)
            } else {
                None
            }
        })
}

#[cfg(feature = "server")]
async fn get_current_admin_user() -> Result<User, ServerFnError> {
    let token = if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
        let parts = ctx.parts_mut();
        parts
            .headers
            .get("cookie")
            .and_then(|h| h.to_str().ok())
            .and_then(parse_session_token)
            .map(|s| s.to_string())
    } else {
        None
    };

    let Some(token) = token else {
        return Err(ServerFnError::new("未登录"));
    };

    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let row = client
        .query_opt(
            "SELECT u.id, u.username, u.email, u.password_hash, u.role, u.created_at
             FROM sessions s
             JOIN users u ON s.user_id = u.id
             WHERE s.token = $1 AND s.expires_at > NOW()",
            &[&token],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?;

    let user = match row {
        Some(row) => {
            let role_str: String = row.get("role");
            let role = UserRole::from_str(&role_str).unwrap_or(UserRole::Blocked);
            User {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                role,
                created_at: row.get("created_at"),
            }
        }
        None => return Err(ServerFnError::new("会话已过期")),
    };

    if user.role != UserRole::Admin {
        return Err(ServerFnError::new("权限不足"));
    }

    Ok(user)
}

// ============================================================================
// Slug utilities
// ============================================================================

#[cfg(feature = "server")]
fn slugify(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();

    // Collapse consecutive dashes
    let parts: Vec<&str> = slug.split('-').filter(|s| !s.is_empty()).collect();
    let slug = parts.join("-");

    // Truncate to 100 chars
    slug.chars().take(100).collect()
}

#[cfg(feature = "server")]
fn is_valid_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > 200 {
        return false;
    }
    slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

#[cfg(feature = "server")]
async fn ensure_unique_slug(
    client: &tokio_postgres::Client,
    base: &str,
    exclude_id: Option<i32>,
) -> Result<String, ServerFnError> {
    let mut candidate = base.to_string();
    let mut suffix = 2;

    loop {
        let exists = if let Some(exclude) = exclude_id {
            client
                .query_opt(
                    "SELECT 1 FROM posts WHERE slug = $1 AND deleted_at IS NULL AND id != $2",
                    &[&candidate, &exclude],
                )
                .await
                .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?
                .is_some()
        } else {
            client
                .query_opt(
                    "SELECT 1 FROM posts WHERE slug = $1 AND deleted_at IS NULL",
                    &[&candidate],
                )
                .await
                .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?
                .is_some()
        };

        if !exists {
            return Ok(candidate);
        }

        candidate = format!("{}-{}", base, suffix);
        suffix += 1;

        if candidate.len() > 200 {
            return Err(ServerFnError::new("无法生成唯一 slug"));
        }
    }
}

// ============================================================================
// Markdown rendering
// ============================================================================

#[cfg(feature = "server")]
fn render_markdown(md: &str) -> String {
    let parser = pulldown_cmark::Parser::new(md);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

#[cfg(feature = "server")]
fn auto_summary(md: &str) -> String {
    // Strip markdown syntax roughly: remove heading markers, bold, italic, links, code fences
    let mut plain = md.to_string();
    // Remove code blocks
    plain = regex::Regex::new(r"```[\s\S]*?```")
        .unwrap()
        .replace_all(&plain, "")
        .to_string();
    // Remove inline code
    plain = regex::Regex::new(r"`[^`]*`").unwrap().replace_all(&plain, "").to_string();
    // Remove links: [text](url) -> text
    plain = regex::Regex::new(r"\[([^\]]*)\]\([^)]*\)")
        .unwrap()
        .replace_all(&plain, "$1")
        .to_string();
    // Remove heading markers
    plain = regex::Regex::new(r"^#{1,6}\s*").unwrap().replace_all(&plain, "").to_string();
    // Remove bold/italic markers
    plain = plain.replace("**", "").replace("*", "").replace("__", "").replace("_", "");
    // Remove images
    plain = regex::Regex::new(r"!\[([^\]]*)\]\([^)]*\)")
        .unwrap()
        .replace_all(&plain, "")
        .to_string();
    // Collapse whitespace
    plain = regex::Regex::new(r"\s+")
        .unwrap()
        .replace_all(&plain, " ")
        .to_string();

    plain.trim().chars().take(200).collect()
}

// ============================================================================
// Tag helpers
// ============================================================================

#[cfg(feature = "server")]
async fn set_post_tags(
    client: &tokio_postgres::Client,
    post_id: i32,
    tags: &[String],
) -> Result<(), ServerFnError> {
    // Remove existing tags
    client
        .execute("DELETE FROM post_tags WHERE post_id = $1", &[&post_id])
        .await
        .map_err(|e| ServerFnError::new(format!("删除标签关联失败: {}", e)))?;

    for tag_name in tags {
        let tag_name = tag_name.trim();
        if tag_name.is_empty() {
            continue;
        }

        // Insert or get tag
        let tag_id: i32 = {
            let row = client
                .query_opt(
                    "INSERT INTO tags (name) VALUES ($1) ON CONFLICT (name) DO NOTHING RETURNING id",
                    &[&tag_name],
                )
                .await
                .map_err(|e| ServerFnError::new(format!("创建标签失败: {}", e)))?;

            match row {
                Some(r) => r.get(0),
                None => {
                    // Tag already exists, fetch its id
                    let row = client
                        .query_opt("SELECT id FROM tags WHERE name = $1", &[&tag_name])
                        .await
                        .map_err(|e| ServerFnError::new(format!("查询标签失败: {}", e)))?;
                    row.map(|r| r.get(0))
                        .ok_or_else(|| ServerFnError::new(format!("标签不存在: {}", tag_name)))?
                }
            }
        };

        client
            .execute(
                "INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2)",
                &[&post_id, &tag_id],
            )
            .await
            .map_err(|e| ServerFnError::new(format!("关联标签失败: {}", e)))?;
    }

    Ok(())
}

#[cfg(feature = "server")]
async fn get_post_tags(client: &tokio_postgres::Client, post_id: i32) -> Vec<String> {
    let rows = client
        .query(
            "SELECT t.name FROM tags t JOIN post_tags pt ON t.id = pt.tag_id WHERE pt.post_id = $1 ORDER BY t.name",
            &[&post_id],
        )
        .await;

    match rows {
        Ok(rows) => rows.iter().map(|r| r.get(0)).collect(),
        Err(_) => vec![],
    }
}

// ============================================================================
// Row to Post conversion
// ============================================================================

#[cfg(feature = "server")]
async fn row_to_post(client: &tokio_postgres::Client, row: &tokio_postgres::Row) -> Post {
    let id: i32 = row.get("id");
    let role_str: String = row.get("status");
    let status = PostStatus::from_str(&role_str).unwrap_or(PostStatus::Draft);
    let tags = get_post_tags(client, id).await;

    Post {
        id,
        author_id: row.get("author_id"),
        title: row.get("title"),
        slug: row.get("slug"),
        summary: row.get("summary"),
        content_md: row.get("content_md"),
        content_html: row.get("content_html"),
        status,
        published_at: row.get("published_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        tags,
    }
}

// ============================================================================
// API Response structs
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub slug: Option<String>,
    pub summary: Option<String>,
    pub content_md: String,
    pub status: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreatePostResponse {
    pub success: bool,
    pub message: String,
    pub post_id: Option<i32>,
    pub slug: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PostListResponse {
    pub posts: Vec<Post>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagListResponse {
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PostStatsResponse {
    pub stats: PostStats,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SinglePostResponse {
    pub post: Option<Post>,
}

// ============================================================================
// Server Functions
// ============================================================================

#[server(CreatePost, "/api")]
pub async fn create_post(
    title: String,
    slug: Option<String>,
    summary: Option<String>,
    content_md: String,
    status: String,
    tags: Vec<String>,
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
        Some(s) if !s.trim().is_empty() => {
            let s = s.trim();
            if !is_valid_slug(s) {
                return Ok(CreatePostResponse {
                    success: false,
                    message: "slug 格式无效，只能包含字母、数字、连字符和下划线".to_string(),
                    post_id: None,
                    slug: None,
                });
            }
            s.to_string()
        }
        _ => slugify(&title),
    };

    let mut client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let final_slug = ensure_unique_slug(&client, &base_slug, None).await?;
    let content_html = render_markdown(&content_md);
    let summary = summary.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| auto_summary(&content_md));
    let post_status = PostStatus::from_str(&status).unwrap_or(PostStatus::Draft);

    let published_at = if post_status == PostStatus::Published {
        Some(chrono::Utc::now())
    } else {
        None
    };

    let tx = client
        .transaction()
        .await
        .map_err(|e| ServerFnError::new(format!("事务开始失败: {}", e)))?;

    let row = tx
        .query_one(
            "INSERT INTO posts (author_id, title, slug, summary, content_md, content_html, status, published_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id",
            &[
                &user.id,
                &title.trim(),
                &final_slug,
                &summary,
                &content_md,
                &content_html,
                &post_status.as_str(),
                &published_at,
            ],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("创建文章失败: {}", e)))?;

    let post_id: i32 = row.get(0);

    // Set tags
    let tags_cleaned: Vec<String> = tags
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    if !tags_cleaned.is_empty() {
        // Use the non-transaction client for tag operations (simpler)
        // Actually we should use the transaction. Let's implement inline.
        for tag_name in &tags_cleaned {
            let tag_id: i32 = {
                let row = tx
                    .query_opt(
                        "INSERT INTO tags (name) VALUES ($1) ON CONFLICT (name) DO NOTHING RETURNING id",
                        &[&tag_name.as_str()],
                    )
                    .await
                    .map_err(|e| ServerFnError::new(format!("创建标签失败: {}", e)))?;

                match row {
                    Some(r) => r.get(0),
                    None => {
                        let row = tx
                            .query_opt("SELECT id FROM tags WHERE name = $1", &[&tag_name.as_str()])
                            .await
                            .map_err(|e| ServerFnError::new(format!("查询标签失败: {}", e)))?;
                        row.map(|r| r.get(0))
                            .ok_or_else(|| ServerFnError::new(format!("标签不存在: {}", tag_name)))?
                    }
                }
            };

            tx.execute(
                "INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2)",
                &[&post_id, &tag_id],
            )
            .await
            .map_err(|e| ServerFnError::new(format!("关联标签失败: {}", e)))?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("事务提交失败: {}", e)))?;

    Ok(CreatePostResponse {
        success: true,
        message: "创建成功".to_string(),
        post_id: Some(post_id),
        slug: Some(final_slug),
    })
}

#[server(UpdatePost, "/api")]
pub async fn update_post(
    post_id: i32,
    title: String,
    slug: Option<String>,
    summary: Option<String>,
    content_md: String,
    status: String,
    tags: Vec<String>,
) -> Result<CreatePostResponse, ServerFnError> {
    let user = get_current_admin_user().await?;

    let mut client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    // Verify ownership
    let exists: bool = client
        .query_opt(
            "SELECT 1 FROM posts WHERE id = $1 AND author_id = $2 AND deleted_at IS NULL",
            &[&post_id, &user.id],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?
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
        Some(s) if !s.trim().is_empty() => {
            let s = s.trim();
            if !is_valid_slug(s) {
                return Ok(CreatePostResponse {
                    success: false,
                    message: "slug 格式无效".to_string(),
                    post_id: None,
                    slug: None,
                });
            }
            s.to_string()
        }
        _ => slugify(&title),
    };

    let final_slug = ensure_unique_slug(&client, &base_slug, Some(post_id)).await?;
    let content_html = render_markdown(&content_md);
    let summary = summary.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| auto_summary(&content_md));
    let post_status = PostStatus::from_str(&status).unwrap_or(PostStatus::Draft);

    let tx = client
        .transaction()
        .await
        .map_err(|e| ServerFnError::new(format!("事务开始失败: {}", e)))?;

    // Check if status changed to published and was not published before
    let old_status_row = tx
        .query_opt(
            "SELECT status, published_at FROM posts WHERE id = $1",
            &[&post_id],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?;

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
        "UPDATE posts SET title = $1, slug = $2, summary = $3, content_md = $4, content_html = $5, status = $6, published_at = $7, updated_at = NOW()
         WHERE id = $8",
        &[
            &title.trim(),
            &final_slug,
            &summary,
            &content_md,
            &content_html,
            &post_status.as_str(),
            &published_at,
            &post_id,
        ],
    )
    .await
    .map_err(|e| ServerFnError::new(format!("更新文章失败: {}", e)))?;

    // Update tags
    let tags_cleaned: Vec<String> = tags
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    tx.execute("DELETE FROM post_tags WHERE post_id = $1", &[&post_id])
        .await
        .map_err(|e| ServerFnError::new(format!("删除旧标签失败: {}", e)))?;

    for tag_name in &tags_cleaned {
        let tag_id: i32 = {
            let row = tx
                .query_opt(
                    "INSERT INTO tags (name) VALUES ($1) ON CONFLICT (name) DO NOTHING RETURNING id",
                    &[&tag_name.as_str()],
                )
                .await
                .map_err(|e| ServerFnError::new(format!("创建标签失败: {}", e)))?;

            match row {
                Some(r) => r.get(0),
                None => {
                    let row = tx
                        .query_opt("SELECT id FROM tags WHERE name = $1", &[&tag_name.as_str()])
                        .await
                        .map_err(|e| ServerFnError::new(format!("查询标签失败: {}", e)))?;
                    row.map(|r| r.get(0))
                        .ok_or_else(|| ServerFnError::new(format!("标签不存在: {}", tag_name)))?
                }
            }
        };

        tx.execute(
            "INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2)",
            &[&post_id, &tag_id],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("关联标签失败: {}", e)))?;
    }

    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("事务提交失败: {}", e)))?;

    Ok(CreatePostResponse {
        success: true,
        message: "更新成功".to_string(),
        post_id: Some(post_id),
        slug: Some(final_slug),
    })
}

#[server(GetPostBySlug, "/api")]
pub async fn get_post_by_slug(slug: String) -> Result<SinglePostResponse, ServerFnError> {
    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let row = client
        .query_opt(
            "SELECT id, author_id, title, slug, summary, content_md, content_html, status, published_at, created_at, updated_at
             FROM posts
             WHERE slug = $1 AND deleted_at IS NULL",
            &[&slug],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?;

    let post = match row {
        Some(row) => Some(row_to_post(&client, &row).await),
        None => None,
    };

    Ok(SinglePostResponse { post })
}

#[server(ListPublishedPosts, "/api")]
pub async fn list_published_posts() -> Result<PostListResponse, ServerFnError> {
    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let rows = client
        .query(
            "SELECT id, author_id, title, slug, summary, content_md, content_html, status, published_at, created_at, updated_at
             FROM posts
             WHERE status = 'published' AND deleted_at IS NULL
             ORDER BY published_at DESC",
            &[],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post(&client, row).await);
    }

    Ok(PostListResponse { posts })
}

#[server(ListPosts, "/api")]
pub async fn list_posts() -> Result<PostListResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let rows = client
        .query(
            "SELECT id, author_id, title, slug, summary, content_md, content_html, status, published_at, created_at, updated_at
             FROM posts
             WHERE deleted_at IS NULL
             ORDER BY created_at DESC",
            &[],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post(&client, row).await);
    }

    Ok(PostListResponse { posts })
}

#[server(DeletePost, "/api")]
pub async fn delete_post(post_id: i32) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let result = client
        .execute(
            "UPDATE posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
            &[&post_id],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("删除失败: {}", e)))?;

    if result == 0 {
        return Ok(CreatePostResponse {
            success: false,
            message: "文章不存在".to_string(),
            post_id: None,
            slug: None,
        });
    }

    Ok(CreatePostResponse {
        success: true,
        message: "删除成功".to_string(),
        post_id: Some(post_id),
        slug: None,
    })
}

#[server(ListTags, "/api")]
pub async fn list_tags() -> Result<TagListResponse, ServerFnError> {
    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let rows = client
        .query(
            "SELECT t.id, t.name, COUNT(pt.post_id) as post_count
             FROM tags t
             LEFT JOIN post_tags pt ON t.id = pt.tag_id
             LEFT JOIN posts p ON pt.post_id = p.id AND p.deleted_at IS NULL AND p.status = 'published'
             GROUP BY t.id, t.name
             ORDER BY t.name",
            &[],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?;

    let tags: Vec<Tag> = rows
        .iter()
        .map(|r| Tag {
            id: r.get("id"),
            name: r.get("name"),
            post_count: r.get("post_count"),
        })
        .collect();

    Ok(TagListResponse { tags })
}

#[server(GetPostsByTag, "/api")]
pub async fn get_posts_by_tag(tag_name: String) -> Result<PostListResponse, ServerFnError> {
    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let rows = client
        .query(
            "SELECT p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, p.status, p.published_at, p.created_at, p.updated_at
             FROM posts p
             JOIN post_tags pt ON p.id = pt.post_id
             JOIN tags t ON pt.tag_id = t.id
             WHERE t.name = $1 AND p.status = 'published' AND p.deleted_at IS NULL
             ORDER BY p.published_at DESC",
            &[&tag_name],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post(&client, row).await);
    }

    Ok(PostListResponse { posts })
}

#[server(GetPostStats, "/api")]
pub async fn get_post_stats() -> Result<PostStatsResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let total: i64 = client
        .query_one("SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL", &[])
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?
        .get(0);

    let drafts: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL AND status = 'draft'",
            &[],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?
        .get(0);

    let published: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL AND status = 'published'",
            &[],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?
        .get(0);

    Ok(PostStatsResponse {
        stats: PostStats {
            total,
            drafts,
            published,
        },
    })
}

#[server(SearchPosts, "/api")]
pub async fn search_posts(query: String) -> Result<PostListResponse, ServerFnError> {
    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let search_pattern = format!("%{}%", query);

    let rows = client
        .query(
            "SELECT p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, p.status, p.published_at, p.created_at, p.updated_at
             FROM posts p
             WHERE p.status = 'published' AND p.deleted_at IS NULL
               AND (p.title ILIKE $1 OR p.content_md ILIKE $1)
             ORDER BY p.published_at DESC",
            &[&search_pattern],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post(&client, row).await);
    }

    Ok(PostListResponse { posts })
}

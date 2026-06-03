#![allow(clippy::unused_unit, deprecated, unused_imports)]

use dioxus::prelude::*;

use crate::db::pool::DB_POOL;
use crate::models::post::{Post, PostStats, PostStatus, Tag};
use crate::models::user::{User, UserRole};

// ============================================================================
// Server-side helpers (only compiled when server feature is enabled)
// ============================================================================

#[cfg(feature = "server")]
fn parse_session_token(cookie_header: &str) -> Option<&str> {
    cookie_header.split(';').map(|s| s.trim()).find_map(|pair| {
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

    let client = DB_POOL.get().await.map_err(|e| {
        tracing::error!("DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

    let row = client
        .query_opt(
            "SELECT u.id, u.username, u.email, u.password_hash, u.role, u.created_at
             FROM sessions s
             JOIN users u ON s.user_id = u.id
             WHERE s.token = $1 AND s.expires_at > NOW()",
            &[&token],
        )
        .await
        .map_err(|e| {
            tracing::error!("query failed: {:?}", e);
            ServerFnError::new(format!("查询失败: {}", e))
        })?;

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
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();

    let parts: Vec<&str> = slug.split('-').filter(|s| !s.is_empty()).collect();
    let slug = parts.join("-");

    if slug.is_empty() {
        return format!("{}", chrono::Utc::now().timestamp());
    }

    slug.chars().take(100).collect()
}

#[cfg(feature = "server")]
fn is_valid_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > 200 {
        return false;
    }
    slug.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
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
// Markdown rendering (enhanced with TOC, word count, reading time, anchors)
// ============================================================================

#[cfg(feature = "server")]
fn clean_html(input: &str) -> String {
    let mut builder = ammonia::Builder::default();
    builder
        .add_generic_attributes(&["class", "aria-hidden", "aria-label", "id", "role", "accesskey", "title"])
        .add_tags(&["details", "summary"])
        .url_relative(ammonia::UrlRelative::PassThrough)
        .add_tag_attributes("a", &["class", "aria-hidden", "aria-label"])
        .add_tag_attributes("h1", &["id", "class"])
        .add_tag_attributes("h2", &["id", "class"])
        .add_tag_attributes("h3", &["id", "class"])
        .add_tag_attributes("h4", &["id", "class"])
        .add_tag_attributes("h5", &["id", "class"])
        .add_tag_attributes("h6", &["id", "class"]);
    
    builder.clean(input).to_string()
}

#[derive(Debug, Clone)]
#[cfg(feature = "server")]
struct RenderedContent {
    html: String,
    toc_html: String,
}

#[cfg(feature = "server")]
fn render_markdown_enhanced(md: &str) -> RenderedContent {
    use pulldown_cmark::{Event, Tag, TagEnd, HeadingLevel};

    // 1. Parse markdown and collect headings for TOC
    let parser = pulldown_cmark::Parser::new(md);
    let mut headings: Vec<(u8, String, String)> = Vec::new(); // (level, text, id)
    let mut current_heading: Option<(u8, String)> = None;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let lvl = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                current_heading = Some((lvl, String::new()));
            }
            Event::Text(text) => {
                if let Some((_, ref mut content)) = current_heading {
                    content.push_str(&text);
                }
            }
            Event::Code(code) => {
                if let Some((_, ref mut content)) = current_heading {
                    content.push_str(&code);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((lvl, text)) = current_heading.take() {
                    let id = slugify_heading(&text);
                    headings.push((lvl, text, id));
                }
            }
            _ => {}
        }
    }

    // 2. Generate TOC HTML
    let toc_html = generate_toc_html(&headings);

    // 3. Generate HTML with heading anchors
    let parser = pulldown_cmark::Parser::new(md);
    let mut html = String::new();
    let mut heading_idx = 0;
    let mut in_heading = false;
    let mut non_heading_events: Vec<Event> = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // Flush non-heading events first
                if !non_heading_events.is_empty() {
                    pulldown_cmark::html::push_html(&mut html, non_heading_events.into_iter());
                    non_heading_events = Vec::new();
                }
                in_heading = true;
                if heading_idx < headings.len() {
                    let (_, _, ref id) = headings[heading_idx];
                    let tag = match level {
                        HeadingLevel::H1 => "h1",
                        HeadingLevel::H2 => "h2",
                        HeadingLevel::H3 => "h3",
                        HeadingLevel::H4 => "h4",
                        HeadingLevel::H5 => "h5",
                        HeadingLevel::H6 => "h6",
                    };
                    html.push_str(&format!("<{} id=\"{}\">", tag, id));
                }
            }
            Event::End(TagEnd::Heading(level)) => {
                if heading_idx < headings.len() {
                    let (_, _, ref id) = headings[heading_idx];
                    let tag = match level {
                        HeadingLevel::H1 => "h1",
                        HeadingLevel::H2 => "h2",
                        HeadingLevel::H3 => "h3",
                        HeadingLevel::H4 => "h4",
                        HeadingLevel::H5 => "h5",
                        HeadingLevel::H6 => "h6",
                    };
                    html.push_str(&format!(
                        "<a class=\"anchor\" aria-hidden=\"true\" href=\"#{}\">#</a></{}>",
                        id, tag
                    ));
                    heading_idx += 1;
                }
                in_heading = false;
            }
            _ => {
                if in_heading {
                    // Manually render heading content
                    match event {
                        Event::Text(text) => html.push_str(&clean_html(&text)),
                        Event::Code(code) => {
                            html.push_str("<code>");
                            html.push_str(&clean_html(&code));
                            html.push_str("</code>");
                        }
                        _ => {}
                    }
                } else {
                    non_heading_events.push(event);
                }
            }
        }
    }

    // Flush remaining non-heading events
    if !non_heading_events.is_empty() {
        pulldown_cmark::html::push_html(&mut html, non_heading_events.into_iter());
    }

    RenderedContent {
        html: clean_html(&html),
        toc_html,
    }
}

#[cfg(feature = "server")]
fn generate_toc_html(headings: &[(u8, String, String)]) -> String {
    if headings.is_empty() {
        return String::new();
    }

    let mut html = String::from("<ul>");
    let mut stack: Vec<u8> = vec![headings[0].0];

    for (i, (level, text, id)) in headings.iter().enumerate() {
        let level = *level;

        if i > 0 {
            let prev_level = headings[i - 1].0;
            if level > prev_level {
                // Open new nested lists
                for _ in prev_level..level {
                    html.push_str("<ul>");
                    stack.push(level);
                }
            } else if level < prev_level {
                // Close nested lists
                while let Some(top) = stack.last() {
                    if *top > level {
                        html.push_str("</li></ul>");
                        stack.pop();
                    } else {
                        break;
                    }
                }
                html.push_str("</li>");
            } else {
                html.push_str("</li>");
            }
        }

        html.push_str(&format!(
            "<li><a href=\"#{}\" aria-label=\"{}\">{}</a>",
            id,
            clean_html(text),
            clean_html(text)
        ));
    }

    // Close remaining lists
    while stack.len() > 1 {
        html.push_str("</li></ul>");
        stack.pop();
    }
    html.push_str("</li></ul>");

    html
}

#[cfg(feature = "server")]
fn slugify_heading(text: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = true;

    for c in text.to_lowercase().chars() {
        if c.is_alphanumeric() {
            slug.push(c);
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }

    if slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        slug.push_str("heading");
    }

    slug
}

#[cfg(feature = "server")]
fn count_words(md: &str) -> u32 {
    // Remove markdown syntax
    let mut plain = md.to_string();
    plain = regex::Regex::new(r"```[\s\S]*?```").unwrap().replace_all(&plain, "").to_string();
    plain = regex::Regex::new(r"`[^`]*`").unwrap().replace_all(&plain, "").to_string();
    plain = regex::Regex::new(r"\[([^\]]*)\]\([^)]*\)").unwrap().replace_all(&plain, "$1").to_string();
    plain = regex::Regex::new(r"^#{1,6}\s*").unwrap().replace_all(&plain, "").to_string();
    plain = regex::Regex::new(r"!\[([^\]]*)\]\([^)]*\)").unwrap().replace_all(&plain, "").to_string();
    plain = plain.replace("**", "").replace("*", "").replace("__", "").replace("_", "");

    // Count Chinese characters and English words
    let mut count = 0u32;
    let mut in_word = false;

    for c in plain.chars() {
        if c as u32 >= 0x4E00 && c as u32 <= 0x9FFF {
            count += 1;
            in_word = false;
        } else if c.is_alphabetic() {
            if !in_word {
                count += 1;
                in_word = true;
            }
        } else {
            in_word = false;
        }
    }

    count.max(1)
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
    plain = regex::Regex::new(r"`[^`]*`")
        .unwrap()
        .replace_all(&plain, "")
        .to_string();
    // Remove links: [text](url) -> text
    plain = regex::Regex::new(r"\[([^\]]*)\]\([^)]*\)")
        .unwrap()
        .replace_all(&plain, "$1")
        .to_string();
    // Remove heading markers
    plain = regex::Regex::new(r"^#{1,6}\s*")
        .unwrap()
        .replace_all(&plain, "")
        .to_string();
    // Remove bold/italic markers
    plain = plain
        .replace("**", "")
        .replace("*", "")
        .replace("__", "")
        .replace("_", "");
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
#[allow(dead_code)]
async fn set_post_tags(
    client: &tokio_postgres::Client,
    post_id: i32,
    tags: &[String],
) -> Result<(), ServerFnError> {
    // Remove existing tags
    client
        .execute("DELETE FROM post_tags WHERE post_id = $1", &[&post_id])
        .await
        .map_err(|e| {
            tracing::error!("delete tag links failed: {:?}", e);
            ServerFnError::new(format!("删除标签关联失败: {}", e))
        })?;

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
                .map_err(|e| {
            tracing::error!("create tag failed: {:?}", e);
            ServerFnError::new(format!("创建标签失败: {}", e))
        })?;

            match row {
                Some(r) => r.get(0),
                None => {
                    // Tag already exists, fetch its id
                    let row = client
                        .query_opt("SELECT id FROM tags WHERE name = $1", &[&tag_name])
                        .await
                        .map_err(|e| {
                            tracing::error!("query tag failed: {:?}", e);
                            ServerFnError::new(format!("查询标签失败: {}", e))
                        })?;
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
            .map_err(|e| {
                tracing::error!("link tag failed: {:?}", e);
                ServerFnError::new(format!("关联标签失败: {}", e))
            })?;
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
async fn row_to_post_list(client: &tokio_postgres::Client, row: &tokio_postgres::Row) -> Post {
    let id: i32 = row.get("id");
    let role_str: String = row.get("status");
    let status = PostStatus::from_str(&role_str).unwrap_or(PostStatus::Draft);
    let tags = get_post_tags(client, id).await;

    let content_md: String = row.get("content_md");
    let word_count = count_words(&content_md);

    Post {
        id,
        author_id: row.get("author_id"),
        title: row.get("title"),
        slug: row.get("slug"),
        summary: row.get("summary"),
        content_md,
        content_html: row.get("content_html"),
        status,
        published_at: row.get("published_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        tags,
        cover_image: row.get("cover_image"),
        reading_time: (word_count / 200).max(1),
        word_count,
        toc_html: None,
        prev_post: None,
        next_post: None,
    }
}

#[cfg(feature = "server")]
async fn row_to_post_full(client: &tokio_postgres::Client, row: &tokio_postgres::Row) -> Post {
    let id: i32 = row.get("id");
    let role_str: String = row.get("status");
    let status = PostStatus::from_str(&role_str).unwrap_or(PostStatus::Draft);
    let tags = get_post_tags(client, id).await;

    let prev_post = if let Ok(prev_title) = row.try_get::<_, String>("prev_title") {
        if let Ok(prev_slug) = row.try_get::<_, String>("prev_slug") {
            Some(crate::models::post::PostNav {
                title: prev_title,
                slug: prev_slug,
            })
        } else {
            None
        }
    } else {
        None
    };

    let next_post = if let Ok(next_title) = row.try_get::<_, String>("next_title") {
        if let Ok(next_slug) = row.try_get::<_, String>("next_slug") {
            Some(crate::models::post::PostNav {
                title: next_title,
                slug: next_slug,
            })
        } else {
            None
        }
    } else {
        None
    };

    let content_md: String = row.get("content_md");
    let word_count = count_words(&content_md);
    let rendered = render_markdown_enhanced(&content_md);

    Post {
        id,
        author_id: row.get("author_id"),
        title: row.get("title"),
        slug: row.get("slug"),
        summary: row.get("summary"),
        content_md,
        content_html: Some(rendered.html),
        status,
        published_at: row.get("published_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        tags,
        cover_image: row.get("cover_image"),
        reading_time: (word_count / 200).max(1),
        word_count,
        toc_html: if rendered.toc_html.is_empty() {
            None
        } else {
            Some(rendered.toc_html)
        },
        prev_post,
        next_post,
    }
}

// ============================================================================
// API Response structs
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct CreatePostRequest {
    pub title: String,
    pub slug: Option<String>,
    pub summary: Option<String>,
    pub content_md: String,
    pub status: String,
    pub tags: Vec<String>,
    pub cover_image: Option<String>,
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

    let mut client = DB_POOL.get().await.map_err(|e| {
        tracing::error!("DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

    let final_slug = ensure_unique_slug(&client, &base_slug, None).await?;
    let rendered = render_markdown_enhanced(&content_md);
    let content_html = rendered.html;
    let summary = summary
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| auto_summary(&content_md));
    let post_status = PostStatus::from_str(&status).unwrap_or(PostStatus::Draft);
    let cover_image = cover_image.filter(|s| !s.trim().is_empty());

    let published_at = if post_status == PostStatus::Published {
        Some(chrono::Utc::now())
    } else {
        None
    };

    let tx = client.transaction().await.map_err(|e| {
        tracing::error!("transaction start failed: {:?}", e);
        ServerFnError::new(format!("事务开始失败: {}", e))
    })?;

    let row = tx
        .query_one(
            "INSERT INTO posts (author_id, title, slug, summary, content_md, content_html, status, published_at, cover_image)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
                &cover_image,
            ],
        )
        .await
        .map_err(|e| {
            tracing::error!("create post failed: {:?}", e);
            ServerFnError::new(format!("创建文章失败: {}", e))
        })?;

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
                    .map_err(|e| {
            tracing::error!("create tag failed: {:?}", e);
            ServerFnError::new(format!("创建标签失败: {}", e))
        })?;

                match row {
                    Some(r) => r.get(0),
                    None => {
                        let row = tx
                            .query_opt("SELECT id FROM tags WHERE name = $1", &[&tag_name.as_str()])
                            .await
                            .map_err(|e| {
                                tracing::error!("query tag failed: {:?}", e);
                                ServerFnError::new(format!("查询标签失败: {}", e))
                            })?;
                        row.map(|r| r.get(0)).ok_or_else(|| {
                            ServerFnError::new(format!("标签不存在: {}", tag_name))
                        })?
                    }
                }
            };

            tx.execute(
                "INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2)",
                &[&post_id, &tag_id],
            )
            .await
            .map_err(|e| {
                tracing::error!("link tag failed: {:?}", e);
                ServerFnError::new(format!("关联标签失败: {}", e))
            })?;
        }
    }

    tx.commit().await.map_err(|e| {
        tracing::error!("transaction commit failed: {:?}", e);
        ServerFnError::new(format!("事务提交失败: {}", e))
    })?;

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
    cover_image: Option<String>,
) -> Result<CreatePostResponse, ServerFnError> {
    let user = get_current_admin_user().await?;

    let mut client = DB_POOL.get().await.map_err(|e| {
        tracing::error!("DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

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
    let rendered = render_markdown_enhanced(&content_md);
    let content_html = rendered.html;
    let summary = summary
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| auto_summary(&content_md));
    let post_status = PostStatus::from_str(&status).unwrap_or(PostStatus::Draft);
    let cover_image = cover_image.filter(|s| !s.trim().is_empty());

    let tx = client.transaction().await.map_err(|e| {
        tracing::error!("transaction start failed: {:?}", e);
        ServerFnError::new(format!("事务开始失败: {}", e))
    })?;

    // Check if status changed to published and was not published before
    let old_status_row = tx
        .query_opt(
            "SELECT status, published_at FROM posts WHERE id = $1",
            &[&post_id],
        )
        .await
        .map_err(|e| {
            tracing::error!("query failed: {:?}", e);
            ServerFnError::new(format!("查询失败: {}", e))
        })?;

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
    .map_err(|e| {
            tracing::error!("update post failed: {:?}", e);
            ServerFnError::new(format!("更新文章失败: {}", e))
        })?;

    // Update tags
    let tags_cleaned: Vec<String> = tags
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    tx.execute("DELETE FROM post_tags WHERE post_id = $1", &[&post_id])
        .await
        .map_err(|e| {
            tracing::error!("delete old tags failed: {:?}", e);
            ServerFnError::new(format!("删除旧标签失败: {}", e))
        })?;

    for tag_name in &tags_cleaned {
        let tag_id: i32 = {
            let row = tx
                .query_opt(
                    "INSERT INTO tags (name) VALUES ($1) ON CONFLICT (name) DO NOTHING RETURNING id",
                    &[&tag_name.as_str()],
                )
                .await
                .map_err(|e| {
            tracing::error!("create tag failed: {:?}", e);
            ServerFnError::new(format!("创建标签失败: {}", e))
        })?;

            match row {
                Some(r) => r.get(0),
                None => {
                    let row = tx
                        .query_opt("SELECT id FROM tags WHERE name = $1", &[&tag_name.as_str()])
                        .await
                        .map_err(|e| {
                            tracing::error!("query tag failed: {:?}", e);
                            ServerFnError::new(format!("查询标签失败: {}", e))
                        })?;
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
        .map_err(|e| {
            tracing::error!("link tag failed: {:?}", e);
            ServerFnError::new(format!("关联标签失败: {}", e))
        })?;
    }

    tx.commit().await.map_err(|e| {
        tracing::error!("transaction commit failed: {:?}", e);
        ServerFnError::new(format!("事务提交失败: {}", e))
    })?;

    Ok(CreatePostResponse {
        success: true,
        message: "更新成功".to_string(),
        post_id: Some(post_id),
        slug: Some(final_slug),
    })
}

#[server(GetPostBySlug, "/api")]
pub async fn get_post_by_slug(slug: String) -> Result<SinglePostResponse, ServerFnError> {
    let client = DB_POOL.get().await.map_err(|e| {
        tracing::error!("DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

    let row = client
        .query_opt(
            "SELECT 
                p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                prev.title as prev_title, prev.slug as prev_slug,
                next.title as next_title, next.slug as next_slug
             FROM posts p
             LEFT JOIN LATERAL (
                 SELECT title, slug FROM posts 
                 WHERE published_at < p.published_at 
                   AND status = 'published' 
                   AND deleted_at IS NULL
                 ORDER BY published_at DESC
                 LIMIT 1
             ) prev ON true
             LEFT JOIN LATERAL (
                 SELECT title, slug FROM posts 
                 WHERE published_at > p.published_at 
                   AND status = 'published' 
                   AND deleted_at IS NULL
                 ORDER BY published_at ASC
                 LIMIT 1
             ) next ON true
             WHERE p.slug = $1 AND p.deleted_at IS NULL",
            &[&slug],
        )
        .await
        .map_err(|e| {
            tracing::error!("query failed: {:?}", e);
            ServerFnError::new(format!("查询失败: {}", e))
        })?;

    let post = match row {
        Some(row) => Some(row_to_post_full(&client, &row).await),
        None => None,
    };

    Ok(SinglePostResponse { post })
}

#[server(ListPublishedPosts, "/api")]
pub async fn list_published_posts(
    page: i32,
    per_page: i32,
) -> Result<PostListResponse, ServerFnError> {
    let client = DB_POOL.get().await.map_err(|e| {
        tracing::error!("DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

    let offset = ((page - 1).max(0) as i64) * (per_page as i64);
    let limit = per_page as i64;
    let rows = client
        .query(
            "SELECT id, author_id, title, slug, summary, content_md, content_html, status, published_at, created_at, updated_at, cover_image
             FROM posts
             WHERE status = 'published' AND deleted_at IS NULL
             ORDER BY published_at DESC
             LIMIT $1 OFFSET $2",
            &[&limit, &offset],
        )
        .await
        .map_err(|e| {
            tracing::error!("query failed: {:?}", e);
            ServerFnError::new(format!("查询失败: {}", e))
        })?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post_list(&client, row).await);
    }

    Ok(PostListResponse { posts })
}

#[server(ListPosts, "/api")]
pub async fn list_posts() -> Result<PostListResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    let client = DB_POOL.get().await.map_err(|e| {
        tracing::error!("DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

    let rows = client
        .query(
            "SELECT id, author_id, title, slug, summary, content_md, content_html, status, published_at, created_at, updated_at, cover_image
             FROM posts
             WHERE deleted_at IS NULL
             ORDER BY created_at DESC",
            &[],
        )
        .await
        .map_err(|e| {
            tracing::error!("query failed: {:?}", e);
            ServerFnError::new(format!("查询失败: {}", e))
        })?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post_list(&client, row).await);
    }

    Ok(PostListResponse { posts })
}

#[server(DeletePost, "/api")]
pub async fn delete_post(post_id: i32) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    let client = DB_POOL.get().await.map_err(|e| {
        tracing::error!("DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

    let result = client
        .execute(
            "UPDATE posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
            &[&post_id],
        )
        .await
        .map_err(|e| {
            tracing::error!("delete failed: {:?}", e);
            ServerFnError::new(format!("删除失败: {}", e))
        })?;

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
    let client = DB_POOL.get().await.map_err(|e| {
        tracing::error!("DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

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
        .map_err(|e| {
            tracing::error!("query failed: {:?}", e);
            ServerFnError::new(format!("查询失败: {}", e))
        })?;

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
    let client = DB_POOL.get().await.map_err(|e| {
        tracing::error!("DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

    let rows = client
        .query(
            "SELECT p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, p.status, p.published_at, p.created_at, p.updated_at, p.cover_image
             FROM posts p
             JOIN post_tags pt ON p.id = pt.post_id
             JOIN tags t ON pt.tag_id = t.id
             WHERE t.name = $1 AND p.status = 'published' AND p.deleted_at IS NULL
             ORDER BY p.published_at DESC",
            &[&tag_name],
        )
        .await
        .map_err(|e| {
            tracing::error!("query failed: {:?}", e);
            ServerFnError::new(format!("查询失败: {}", e))
        })?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post_list(&client, row).await);
    }

    Ok(PostListResponse { posts })
}

#[server(GetPostStats, "/api")]
pub async fn get_post_stats() -> Result<PostStatsResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    let client = DB_POOL.get().await.map_err(|e| {
        tracing::error!("DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

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
    let client = DB_POOL.get().await.map_err(|e| {
        tracing::error!("DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

    let search_pattern = format!("%{}%", query);

    let rows = client
        .query(
            "SELECT p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, p.status, p.published_at, p.created_at, p.updated_at, p.cover_image
             FROM posts p
             WHERE p.status = 'published' AND p.deleted_at IS NULL
               AND (p.title ILIKE $1 OR p.content_md ILIKE $1)
             ORDER BY p.published_at DESC",
            &[&search_pattern],
        )
        .await
        .map_err(|e| {
            tracing::error!("query failed: {:?}", e);
            ServerFnError::new(format!("查询失败: {}", e))
        })?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post_list(&client, row).await);
    }

    Ok(PostListResponse { posts })
}
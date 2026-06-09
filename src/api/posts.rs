#![allow(clippy::unused_unit, deprecated, unused_imports)]

use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::api::utils::{db_conn_error, query_error, tx_error};
#[cfg(feature = "server")]
use crate::auth::session::get_session_from_ctx;
use crate::db::pool::get_conn;
use crate::models::post::{Post, PostStats, PostStatus, Tag};
use crate::models::user::{User, UserRole};
#[cfg(feature = "server")]
use crate::utils::text::{auto_summary, count_words};
#[cfg(feature = "server")]
use crate::cache;

// Re-export extracted modules
#[cfg(feature = "server")]
pub use crate::api::markdown::render_markdown_enhanced;
#[cfg(feature = "server")]
pub use crate::api::slug::{ensure_unique_slug, is_valid_slug, slugify};


// ============================================================================
// Server-side helpers (only compiled when server feature is enabled)
// ============================================================================

#[cfg(feature = "server")]
async fn get_current_admin_user() -> Result<User, ServerFnError> {
    let token = match get_session_from_ctx() {
        Some(t) => t,
        None => return Err(ServerFnError::new("未登录")),
    };

    let user = match crate::api::auth::get_user_by_token(&token).await? {
        Some(u) => u,
        None => return Err(ServerFnError::new("会话已过期")),
    };

    if user.role != UserRole::Admin {
        return Err(ServerFnError::new("权限不足"));
    }

    Ok(user)
}

// ============================================================================
// Row to Post conversion
// ============================================================================

#[cfg(feature = "server")]
async fn row_to_post_list(_client: &tokio_postgres::Client, row: &tokio_postgres::Row) -> Post {
    let id: i32 = row.get("id");
    let role_str: String = row.get("status");
    let status = PostStatus::from_str(&role_str).unwrap_or(PostStatus::Draft);
    
    let tags: Vec<String> = row
        .try_get::<_, Vec<String>>("tags")
        .unwrap_or_default()
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect();

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
async fn row_to_post_full(_client: &tokio_postgres::Client, row: &tokio_postgres::Row) -> Post {
    let id: i32 = row.get("id");
    let role_str: String = row.get("status");
    let status = PostStatus::from_str(&role_str).unwrap_or(PostStatus::Draft);
    
    let tags: Vec<String> = row
        .try_get::<_, Vec<String>>("tags")
        .unwrap_or_default()
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect();

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
    let rendered = crate::api::markdown::render_markdown_enhanced(&content_md);

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

    let mut client = get_conn().await.map_err(db_conn_error)?;

    let final_slug = crate::api::slug::ensure_unique_slug(&client, &base_slug, None).await?;
    let rendered = crate::api::markdown::render_markdown_enhanced(&content_md);
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

    let tx = client.transaction().await.map_err(tx_error)?;

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

    let tags_cleaned: Vec<String> = tags
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    if !tags_cleaned.is_empty() {
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

    tx.commit().await.map_err(tx_error)?;

    // Invalidate caches after successful creation
    #[cfg(feature = "server")]
    {
        cache::invalidate_post_lists();
        cache::invalidate_all_tags();
        cache::invalidate_post_stats();
    }

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

    let mut client = get_conn().await.map_err(db_conn_error)?;

    let exists: bool = client
        .query_opt(
            "SELECT 1 FROM posts WHERE id = $1 AND author_id = $2 AND deleted_at IS NULL",
            &[&post_id, &user.id],
        )
        .await
        .map_err(query_error)?
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
        .unwrap_or_else(|| auto_summary(&content_md));
    let post_status = PostStatus::from_str(&status).unwrap_or(PostStatus::Draft);
    let cover_image = cover_image.filter(|s| !s.trim().is_empty());

    let tx = client.transaction().await.map_err(tx_error)?;

    let old_status_row = tx
        .query_opt(
            "SELECT status, published_at FROM posts WHERE id = $1",
            &[&post_id],
        )
        .await
        .map_err(query_error)?;

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

    tx.commit().await.map_err(tx_error)?;

    // Invalidate caches after successful update
    #[cfg(feature = "server")]
    {
        cache::invalidate_post_lists();
        cache::invalidate_all_tags();
        cache::invalidate_post_by_slug(&final_slug).await;
        cache::invalidate_post_stats();

        // Invalidate tag-specific caches for new tags
        for tag_name in &tags_cleaned {
            cache::invalidate_posts_by_tag(tag_name).await;
        }
    }

    Ok(CreatePostResponse {
        success: true,
        message: "更新成功".to_string(),
        post_id: Some(post_id),
        slug: Some(final_slug),
    })
}

#[server(GetPostById, "/api")]
pub async fn get_post_by_id(post_id: i32) -> Result<SinglePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    let client = get_conn().await.map_err(db_conn_error)?;

    let row = client
        .query_opt(
            "SELECT 
                p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as tags
             FROM posts p
             LEFT JOIN post_tags pt ON p.id = pt.post_id
             LEFT JOIN tags t ON pt.tag_id = t.id
             WHERE p.id = $1 AND p.deleted_at IS NULL
             GROUP BY p.id",
            &[&post_id],
        )
        .await
        .map_err(query_error)?;

    let post = match row {
        Some(row) => Some(row_to_post_list(&client, &row).await),
        None => None,
    };

    Ok(SinglePostResponse { post })
}

#[server(GetPostBySlug, "/api")]
pub async fn get_post_by_slug(slug: String) -> Result<SinglePostResponse, ServerFnError> {
    if let Some(cached) = cache::get_post_by_slug(&slug).await {
        return Ok(SinglePostResponse { post: cached });
    }

    let client = get_conn().await.map_err(db_conn_error)?;

    let row = client
        .query_opt(
            "SELECT 
                p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as tags,
                prev.title as prev_title, prev.slug as prev_slug,
                next.title as next_title, next.slug as next_slug
             FROM posts p
             LEFT JOIN post_tags pt ON p.id = pt.post_id
             LEFT JOIN tags t ON pt.tag_id = t.id
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
             WHERE p.slug = $1 AND p.deleted_at IS NULL
             GROUP BY p.id, prev.title, prev.slug, next.title, next.slug",
            &[&slug],
        )
        .await
        .map_err(query_error)?;

    let post = match row {
        Some(row) => Some(row_to_post_full(&client, &row).await),
        None => None,
    };

    if post.is_some() {
        cache::set_post_by_slug(&slug, post.clone()).await;
    }
    Ok(SinglePostResponse { post })
}

#[server(ListPublishedPosts, "/api")]
pub async fn list_published_posts(
    page: i32,
    per_page: i32,
) -> Result<PostListResponse, ServerFnError> {
    let cache_key = cache::CacheKey::PublishedPosts { page, per_page };
    if let Some(cached) = cache::get_post_list(&cache_key).await {
        return Ok(PostListResponse { posts: cached });
    }

    let client = get_conn().await.map_err(db_conn_error)?;

    let offset = ((page - 1).max(0) as i64) * (per_page as i64);
    let limit = per_page as i64;
    let rows = client
        .query(
            "SELECT 
                p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as tags
             FROM posts p
             LEFT JOIN post_tags pt ON p.id = pt.post_id
             LEFT JOIN tags t ON pt.tag_id = t.id
             WHERE p.status = 'published' AND p.deleted_at IS NULL
             GROUP BY p.id
             ORDER BY p.published_at DESC
             LIMIT $1 OFFSET $2",
            &[&limit, &offset],
        )
        .await
        .map_err(query_error)?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post_list(&client, row).await);
    }

    cache::set_post_list(&cache_key, posts.clone()).await;
    Ok(PostListResponse { posts })
}

#[server(ListPosts, "/api")]
pub async fn list_posts() -> Result<PostListResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    let client = get_conn().await.map_err(db_conn_error)?;

    let rows = client
        .query(
            "SELECT 
                p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as tags
             FROM posts p
             LEFT JOIN post_tags pt ON p.id = pt.post_id
             LEFT JOIN tags t ON pt.tag_id = t.id
             WHERE p.deleted_at IS NULL
             GROUP BY p.id
             ORDER BY p.created_at DESC",
            &[],
        )
        .await
        .map_err(query_error)?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post_list(&client, row).await);
    }

    Ok(PostListResponse { posts })
}

#[server(DeletePost, "/api")]
pub async fn delete_post(post_id: i32) -> Result<CreatePostResponse, ServerFnError> {
    let _user = get_current_admin_user().await?;

    let client = get_conn().await.map_err(db_conn_error)?;

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

    // Invalidate all post-related caches
    #[cfg(feature = "server")]
    {
        cache::invalidate_all_post_caches();
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
    if let Some(cached) = cache::get_tag_list().await {
        return Ok(TagListResponse { tags: cached });
    }

    let client = get_conn().await.map_err(db_conn_error)?;

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
        .map_err(query_error)?;

    let tags: Vec<Tag> = rows
        .iter()
        .map(|r| Tag {
            id: r.get("id"),
            name: r.get("name"),
            post_count: r.get("post_count"),
        })
        .collect();

    cache::set_tag_list(tags.clone()).await;
    Ok(TagListResponse { tags })
}

#[server(GetPostsByTag, "/api")]
pub async fn get_posts_by_tag(tag_name: String) -> Result<PostListResponse, ServerFnError> {
    if let Some(cached) = cache::get_posts_by_tag(&tag_name).await {
        return Ok(PostListResponse { posts: cached });
    }

    let client = get_conn().await.map_err(db_conn_error)?;

    let rows = client
        .query(
            "SELECT 
                p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                COALESCE(array_agg(t2.name) FILTER (WHERE t2.name IS NOT NULL), '{}') as tags
             FROM posts p
             JOIN post_tags pt ON p.id = pt.post_id
             JOIN tags t ON pt.tag_id = t.id
             LEFT JOIN post_tags pt2 ON p.id = pt2.post_id
             LEFT JOIN tags t2 ON pt2.tag_id = t2.id
             WHERE t.name = $1 AND p.status = 'published' AND p.deleted_at IS NULL
             GROUP BY p.id
             ORDER BY p.published_at DESC",
            &[&tag_name],
        )
        .await
        .map_err(query_error)?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post_list(&client, row).await);
    }

    cache::set_posts_by_tag(&tag_name, posts.clone()).await;
    Ok(PostListResponse { posts })
}

#[server(GetPostStats, "/api")]
pub async fn get_post_stats() -> Result<PostStatsResponse, ServerFnError> {
    if let Some(cached) = cache::get_post_stats().await {
        return Ok(PostStatsResponse { stats: cached });
    }

    let _user = get_current_admin_user().await?;

    let client = get_conn().await.map_err(db_conn_error)?;

    let total: i64 = client
        .query_one("SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL", &[])
        .await
        .map_err(query_error)?
        .get(0);

    let drafts: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL AND status = 'draft'",
            &[],
        )
        .await
        .map_err(query_error)?
        .get(0);

    let published: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL AND status = 'published'",
            &[],
        )
        .await
        .map_err(query_error)?
        .get(0);

    let stats = PostStats {
        total,
        drafts,
        published,
    };
    cache::set_post_stats(stats.clone()).await;
    Ok(PostStatsResponse { stats })
}

#[server(SearchPosts, "/api")]
pub async fn search_posts(query: String) -> Result<PostListResponse, ServerFnError> {
    let client = get_conn().await.map_err(db_conn_error)?;

    let q = query.trim();
    if q.is_empty() {
        return Ok(PostListResponse { posts: Vec::new() });
    }

    let rows = client
        .query(
            "SELECT 
                p.id, p.author_id, p.title, p.slug, p.summary, p.content_md, p.content_html, 
                p.status, p.published_at, p.created_at, p.updated_at, p.cover_image,
                COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as tags,
                word_similarity(p.search_text, $1) AS sml
             FROM posts p
             LEFT JOIN post_tags pt ON p.id = pt.post_id
             LEFT JOIN tags t ON pt.tag_id = t.id
             WHERE p.status = 'published' AND p.deleted_at IS NULL
               AND p.search_text ILIKE '%' || $1 || '%'
             GROUP BY p.id, p.search_text
             ORDER BY sml DESC, p.published_at DESC
             LIMIT 50",
            &[&q],
        )
        .await
        .map_err(query_error)?;

    let mut posts = Vec::new();
    for row in &rows {
        posts.push(row_to_post_list(&client, row).await);
    }

    Ok(PostListResponse { posts })
}

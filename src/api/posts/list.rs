//! 文章列表查询接口。
//!
//! 提供已发布文章分页、管理员全量列表、以及按标签筛选三种查询能力，
//! 均通过缓存层减少重复数据库访问。
//! Dioxus server function，注册在 `/api` 路径下。
//! 仅在 `feature = "server"` 启用的服务端构建中查询数据库。

use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::helpers::{get_current_admin_user, row_to_post_list};
use super::types::PostListResponse;
#[cfg(feature = "server")]
use crate::api::error::AppError;
use crate::db::pool::get_conn;

/// 单页允许的最大文章数。
///
/// 公开的 `list_published_posts` 接口无需认证，若不对 `per_page` 设上限，
/// 攻击者可传入巨大值迫使数据库扫描并实例化超大 Vec，造成内存放大与拒绝服务。
const MAX_PER_PAGE: i32 = 50;

/// 允许的最大页码。
///
/// `page` 无上限时，攻击者可用海量不同 `page` 值撑大缓存键空间（缓存污染），
/// 并触发无意义的超大 `OFFSET` 扫描。10_000 对任何实际博客都足够宽裕
/// （配合 `MAX_PER_PAGE` 最多覆盖 50 万篇文章），同时把缓存键空间限制在有限范围。
const MAX_PAGE: i32 = 10_000;

/// 将分页参数钳制到安全范围：页码 1–`MAX_PAGE`，每页 1–`MAX_PER_PAGE`。
///
/// 注意：返回值必须同时用于缓存键与 SQL 查询，避免同一逻辑页落入不同缓存条目。
fn clamp_pagination(page: i32, per_page: i32) -> (i32, i32) {
    (
        page.clamp(1, MAX_PAGE),
        per_page.clamp(1, MAX_PER_PAGE),
    )
}

/// 获取已发布文章分页列表。
///
/// 优先命中缓存；未命中时查询总数与分页记录，并按 published_at 降序排列。
#[server(ListPublishedPosts, "/api")]
pub async fn list_published_posts(
    page: i32,
    per_page: i32,
) -> Result<PostListResponse, ServerFnError> {
    // 钳制分页参数，防止无认证调用方请求超大每页数量导致内存放大 / DoS。
    let (page, per_page) = clamp_pagination(page, per_page);

    #[cfg(feature = "server")]
    {
        let cache_key = crate::cache::CacheKey::PublishedPosts { page, per_page };
        if let Some((cached_posts, cached_total)) = crate::cache::get_post_list(&cache_key).await {
            return Ok(PostListResponse {
                posts: cached_posts,
                total: cached_total,
            });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        // 优先读取缓存中的已发布文章总数，否则查询数据库并回填缓存。
        let total = if let Some(cached_total) = crate::cache::get_total_published_posts().await {
            cached_total
        } else {
            let count_row = client
                .query_one(
                    "SELECT COUNT(*) FROM posts WHERE status = 'published' AND deleted_at IS NULL",
                    &[],
                )
                .await
                .map_err(AppError::query)?;
            let total: i64 = count_row.get(0);
            crate::cache::set_total_published_posts(total).await;
            total
        };

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
            .map_err(AppError::query)?;

        let mut posts = Vec::new();
        for row in &rows {
            posts.push(row_to_post_list(&client, row).await);
        }

        crate::cache::set_post_list(&cache_key, posts.clone(), total).await;
        Ok(PostListResponse { posts, total })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(PostListResponse {
            posts: Vec::new(),
            total: 0,
        })
    }
}

/// 获取管理员视角的全部文章列表（含草稿与已发布）。
///
/// 需要 admin 权限；结果按创建时间降序，不走缓存。
#[server(ListPosts, "/api")]
pub async fn list_posts(page: i32, per_page: i32) -> Result<PostListResponse, ServerFnError> {
    // 与公开接口保持一致的分页钳制，避免单次请求拉取过多记录。
    let (page, per_page) = clamp_pagination(page, per_page);
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let count_row = client
            .query_one("SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL", &[])
            .await
            .map_err(AppError::query)?;
        let total: i64 = count_row.get(0);

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
                 WHERE p.deleted_at IS NULL
                 GROUP BY p.id
                 ORDER BY p.created_at DESC
                 LIMIT $1 OFFSET $2",
                &[&limit, &offset],
            )
            .await
            .map_err(AppError::query)?;

        let mut posts = Vec::new();
        for row in &rows {
            posts.push(row_to_post_list(&client, row).await);
        }

        Ok(PostListResponse { posts, total })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(PostListResponse {
            posts: Vec::new(),
            total: 0,
        })
    }
}

/// 获取指定标签下的已发布文章列表。
///
/// 优先命中缓存；当前实现返回全部匹配文章，因此 total 用 posts.len() 计算。
#[server(GetPostsByTag, "/api")]
pub async fn get_posts_by_tag(tag_name: String) -> Result<PostListResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if let Some((cached_posts, cached_total)) = crate::cache::get_posts_by_tag(&tag_name).await
        {
            return Ok(PostListResponse {
                posts: cached_posts,
                total: cached_total,
            });
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        // 通过 JOIN 筛选含目标标签的已发布文章，并聚合该文章的所有标签。
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
            .map_err(AppError::query)?;

        let mut posts = Vec::new();
        for row in &rows {
            posts.push(row_to_post_list(&client, row).await);
        }

        // 当前查询未分页，返回全部匹配文章，因此 total 等于结果长度。
        // 若后续增加分页，应改为 COUNT(*) 查询。
        let total = posts.len() as i64;
        crate::cache::set_posts_by_tag(&tag_name, posts.clone(), total).await;
        Ok(PostListResponse { posts, total })
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(PostListResponse {
            posts: Vec::new(),
            total: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_pagination_keeps_valid_values() {
        assert_eq!(clamp_pagination(1, 10), (1, 10));
        assert_eq!(clamp_pagination(3, 20), (3, 20));
    }

    #[test]
    fn clamp_pagination_clamps_oversized_per_page() {
        // 攻击者传入超大 per_page 必须被压回上限，避免内存放大 / DoS。
        assert_eq!(clamp_pagination(1, 1_000_000_000), (1, MAX_PER_PAGE));
        assert_eq!(clamp_pagination(2, 51), (2, MAX_PER_PAGE));
    }

    #[test]
    fn clamp_pagination_clamps_non_positive() {
        assert_eq!(clamp_pagination(0, 10), (1, 10));
        assert_eq!(clamp_pagination(-5, 10), (1, 10));
        assert_eq!(clamp_pagination(1, 0), (1, 1));
        assert_eq!(clamp_pagination(1, -100), (1, 1));
    }

    #[test]
    fn clamp_pagination_clamps_oversized_page() {
        // 巨大 page 必须被压回上限，避免无界 OFFSET 扫描与缓存键扇出。
        assert_eq!(clamp_pagination(i32::MAX, 10), (MAX_PAGE, 10));
        assert_eq!(clamp_pagination(MAX_PAGE + 1, 10), (MAX_PAGE, 10));
    }

    #[test]
    fn clamp_pagination_max_page_boundary() {
        assert_eq!(clamp_pagination(MAX_PAGE, 10), (MAX_PAGE, 10));
        assert_eq!(clamp_pagination(MAX_PAGE - 1, 10), (MAX_PAGE - 1, 10));
    }

    #[test]
    fn clamp_pagination_max_per_page_boundary() {
        assert_eq!(clamp_pagination(1, MAX_PER_PAGE), (1, MAX_PER_PAGE));
        assert_eq!(
            clamp_pagination(1, MAX_PER_PAGE - 1),
            (1, MAX_PER_PAGE - 1)
        );
    }
}

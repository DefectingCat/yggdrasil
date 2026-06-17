//! 基于 moka 的内存缓存层。
//!
//! 仅在启用 `server` feature 时编译，为文章列表、标签、单篇文章、统计信息
//! 以及评论相关数据提供按键缓存与失效能力。
//! 缓存使用 `std::sync::LazyLock` 全局实例，按不同业务数据设置独立的 TTL。

#[cfg(feature = "server")]
use moka::future::Cache;
#[cfg(feature = "server")]
use std::sync::LazyLock;
#[cfg(feature = "server")]
use std::time::Duration;

#[cfg(feature = "server")]
use crate::models::comment::PublicComment;
#[cfg(feature = "server")]
use crate::models::post::{Post, PostListItem, PostStats, Tag};

// ============================================================================
// 缓存 TTL 配置
// ============================================================================

/// 文章列表缓存 TTL：60 秒。
#[cfg(feature = "server")]
const TTL_POST_LIST: Duration = Duration::from_secs(60);

/// 标签列表缓存 TTL：300 秒。
#[cfg(feature = "server")]
const TTL_TAG_LIST: Duration = Duration::from_secs(300);

/// 单篇文章缓存 TTL：600 秒。
#[cfg(feature = "server")]
const TTL_SINGLE_POST: Duration = Duration::from_secs(600);

/// 文章统计缓存 TTL：60 秒。
#[cfg(feature = "server")]
const TTL_POST_STATS: Duration = Duration::from_secs(60);

/// 标签下文章列表缓存 TTL：120 秒。
#[cfg(feature = "server")]
const TTL_TAG_POSTS: Duration = Duration::from_secs(120);

/// 评论列表缓存 TTL：60 秒。
#[cfg(feature = "server")]
const TTL_COMMENTS: Duration = Duration::from_secs(60);

/// 待审核评论数量缓存 TTL：10 秒，因管理后台需要较实时数据。
#[cfg(feature = "server")]
const TTL_PENDING_COUNT: Duration = Duration::from_secs(10);

// ============================================================================
// 缓存 Key 类型
// ============================================================================

/// 统一的缓存键枚举，每个变体对应一类可缓存数据。
#[cfg(feature = "server")]
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum CacheKey {
    /// 已发布文章分页列表。
    PublishedPosts { page: i32, per_page: i32 },
    /// 已发布文章总数。
    TotalPublishedPosts,
    /// 全部标签。
    AllTags,
    /// 按 slug 查询的单篇文章。
    PostBySlug(String),
    /// 按标签查询的文章列表。
    PostsByTag(String),
    /// 文章统计信息。
    PostStats,
    /// 某篇文章下的评论列表。
    CommentsByPost { post_id: i32 },
    /// 待审核评论总数。
    PendingCommentCount,
}

// ============================================================================
// 缓存实例
// ============================================================================

/// 文章列表缓存类型，值为（文章列表，总数）。
#[cfg(feature = "server")]
pub type PostListCache = Cache<CacheKey, (Vec<PostListItem>, i64)>;

/// 标签列表缓存类型。
#[cfg(feature = "server")]
pub type TagListCache = Cache<CacheKey, Vec<Tag>>;

/// 单篇文章缓存类型。
#[cfg(feature = "server")]
pub type SinglePostCache = Cache<CacheKey, Option<Post>>;

/// 文章统计缓存类型。
#[cfg(feature = "server")]
pub type PostStatsCache = Cache<CacheKey, PostStats>;

/// 全局文章列表缓存实例，最大容量 100。
#[cfg(feature = "server")]
static POST_LIST_CACHE: LazyLock<PostListCache> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(100)
        .time_to_live(TTL_POST_LIST)
        .build()
});

/// 全局标签列表缓存实例，最大容量 50。
#[cfg(feature = "server")]
static TAG_LIST_CACHE: LazyLock<TagListCache> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(50)
        .time_to_live(TTL_TAG_LIST)
        .build()
});

/// 全局单篇文章缓存实例，最大容量 200。
#[cfg(feature = "server")]
static SINGLE_POST_CACHE: LazyLock<SinglePostCache> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(200)
        .time_to_live(TTL_SINGLE_POST)
        .build()
});

/// 全局文章统计缓存实例，最大容量 10。
#[cfg(feature = "server")]
static POST_STATS_CACHE: LazyLock<PostStatsCache> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(10)
        .time_to_live(TTL_POST_STATS)
        .build()
});

/// 全局标签文章列表缓存实例，最大容量 100。
#[cfg(feature = "server")]
static TAG_POSTS_CACHE: LazyLock<PostListCache> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(100)
        .time_to_live(TTL_TAG_POSTS)
        .build()
});

/// 评论列表缓存类型。
#[cfg(feature = "server")]
pub type CommentListCache = Cache<CacheKey, Vec<PublicComment>>;

/// 全局评论列表缓存实例，最大容量 200。
#[cfg(feature = "server")]
static COMMENT_CACHE: LazyLock<CommentListCache> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(200)
        .time_to_live(TTL_COMMENTS)
        .build()
});

/// 全局待审核评论数量缓存实例，最大容量 10。
#[cfg(feature = "server")]
static PENDING_COUNT_CACHE: LazyLock<Cache<CacheKey, i64>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(10)
        .time_to_live(TTL_PENDING_COUNT)
        .build()
});

// ============================================================================
// 公共缓存 API
// ============================================================================

/// 读取文章分页列表缓存。
#[cfg(feature = "server")]
pub async fn get_post_list(key: &CacheKey) -> Option<(Vec<PostListItem>, i64)> {
    POST_LIST_CACHE.get(key).await
}

/// 写入文章分页列表缓存。
#[cfg(feature = "server")]
pub async fn set_post_list(key: &CacheKey, posts: Vec<PostListItem>, total: i64) {
    let _ = POST_LIST_CACHE.insert(key.clone(), (posts, total)).await;
}

/// 读取已发布文章总数缓存。
#[cfg(feature = "server")]
pub async fn get_total_published_posts() -> Option<i64> {
    POST_LIST_CACHE
        .get(&CacheKey::TotalPublishedPosts)
        .await
        .map(|(_, total)| total)
}

/// 写入已发布文章总数缓存，文章列表部分置空以节省内存。
#[cfg(feature = "server")]
pub async fn set_total_published_posts(total: i64) {
    let _ = POST_LIST_CACHE
        .insert(CacheKey::TotalPublishedPosts, (vec![], total))
        .await;
}

/// 读取全部标签缓存。
#[cfg(feature = "server")]
pub async fn get_tag_list() -> Option<Vec<Tag>> {
    TAG_LIST_CACHE.get(&CacheKey::AllTags).await
}

/// 写入全部标签缓存。
#[cfg(feature = "server")]
pub async fn set_tag_list(tags: Vec<Tag>) {
    let _ = TAG_LIST_CACHE.insert(CacheKey::AllTags, tags).await;
}

/// 按 slug 读取单篇文章缓存。
#[cfg(feature = "server")]
pub async fn get_post_by_slug(slug: &str) -> Option<Option<Post>> {
    SINGLE_POST_CACHE
        .get(&CacheKey::PostBySlug(slug.to_string()))
        .await
}

/// 按 slug 写入单篇文章缓存，None 表示文章不存在。
#[cfg(feature = "server")]
pub async fn set_post_by_slug(slug: &str, post: Option<Post>) {
    let _ = SINGLE_POST_CACHE
        .insert(CacheKey::PostBySlug(slug.to_string()), post)
        .await;
}

/// 按标签读取文章列表缓存。
#[cfg(feature = "server")]
pub async fn get_posts_by_tag(tag: &str) -> Option<(Vec<PostListItem>, i64)> {
    TAG_POSTS_CACHE
        .get(&CacheKey::PostsByTag(tag.to_string()))
        .await
}

/// 按标签写入文章列表缓存。
#[cfg(feature = "server")]
pub async fn set_posts_by_tag(tag: &str, posts: Vec<PostListItem>, total: i64) {
    let _ = TAG_POSTS_CACHE
        .insert(CacheKey::PostsByTag(tag.to_string()), (posts, total))
        .await;
}

/// 读取文章统计缓存。
#[cfg(feature = "server")]
pub async fn get_post_stats() -> Option<PostStats> {
    POST_STATS_CACHE.get(&CacheKey::PostStats).await
}

/// 写入文章统计缓存。
#[cfg(feature = "server")]
pub async fn set_post_stats(stats: PostStats) {
    let _ = POST_STATS_CACHE.insert(CacheKey::PostStats, stats).await;
}

// ============================================================================
// 缓存失效
// ============================================================================

/// 清空所有文章分页列表缓存。
#[cfg(feature = "server")]
pub fn invalidate_post_lists() {
    POST_LIST_CACHE.invalidate_all();
}

/// 清空所有标签缓存。
#[cfg(feature = "server")]
pub fn invalidate_all_tags() {
    TAG_LIST_CACHE.invalidate_all();
}

/// 按 slug 失效单篇文章缓存。
#[cfg(feature = "server")]
pub async fn invalidate_post_by_slug(slug: &str) {
    SINGLE_POST_CACHE
        .invalidate(&CacheKey::PostBySlug(slug.to_string()))
        .await;
}

/// 按标签失效文章列表缓存。
#[cfg(feature = "server")]
pub async fn invalidate_posts_by_tag(tag: &str) {
    TAG_POSTS_CACHE
        .invalidate(&CacheKey::PostsByTag(tag.to_string()))
        .await;
}

/// 清空文章统计缓存。
#[cfg(feature = "server")]
pub fn invalidate_post_stats() {
    POST_STATS_CACHE.invalidate_all();
}

/// 按标签批量失效文章列表缓存。
#[cfg(feature = "server")]
pub async fn invalidate_tag_posts_for(tags: &[String]) {
    for tag in tags {
        invalidate_posts_by_tag(tag).await;
    }
}

/// 清空所有文章相关缓存（列表、标签、单篇、统计、标签文章）。
///
/// 这是一个“紧急”使用的全量失效开关，会一次性冲刷所有文章缓存；
/// 正常写路径应当使用更细粒度的 `invalidate_post_lists` / `invalidate_all_tags` /
/// `invalidate_post_by_slug` / `invalidate_posts_by_tag` / `invalidate_post_stats` /
/// `invalidate_tag_posts_for` 等函数，避免不必要的缓存击穿。
#[cfg(feature = "server")]
pub fn invalidate_all_post_caches() {
    POST_LIST_CACHE.invalidate_all();
    TAG_LIST_CACHE.invalidate_all();
    SINGLE_POST_CACHE.invalidate_all();
    POST_STATS_CACHE.invalidate_all();
    TAG_POSTS_CACHE.invalidate_all();
}

/// 按文章主键读取评论列表缓存。
#[cfg(feature = "server")]
pub async fn get_comments_by_post(post_id: i32) -> Option<Vec<PublicComment>> {
    COMMENT_CACHE
        .get(&CacheKey::CommentsByPost { post_id })
        .await
}

/// 按文章主键写入评论列表缓存。
#[cfg(feature = "server")]
pub async fn set_comments_by_post(post_id: i32, comments: Vec<PublicComment>) {
    let _ = COMMENT_CACHE
        .insert(CacheKey::CommentsByPost { post_id }, comments)
        .await;
}

/// 读取待审核评论总数缓存。
#[cfg(feature = "server")]
pub async fn get_pending_count() -> Option<i64> {
    PENDING_COUNT_CACHE
        .get(&CacheKey::PendingCommentCount)
        .await
}

/// 写入待审核评论总数缓存。
#[cfg(feature = "server")]
pub async fn set_pending_count(count: i64) {
    let _ = PENDING_COUNT_CACHE
        .insert(CacheKey::PendingCommentCount, count)
        .await;
}

/// 按文章主键失效评论列表缓存。
#[cfg(feature = "server")]
pub async fn invalidate_comments_by_post(post_id: i32) {
    COMMENT_CACHE
        .invalidate(&CacheKey::CommentsByPost { post_id })
        .await;
}

/// 失效待审核评论总数缓存。
#[cfg(feature = "server")]
pub async fn invalidate_pending_count() {
    PENDING_COUNT_CACHE
        .invalidate(&CacheKey::PendingCommentCount)
        .await;
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::models::comment::PublicComment;
    use crate::models::post::PostStatus;
    use serial_test::serial;

    #[test]
    #[serial]
    fn cache_key_equality() {
        let k1 = CacheKey::PublishedPosts {
            page: 1,
            per_page: 10,
        };
        let k2 = CacheKey::PublishedPosts {
            page: 1,
            per_page: 10,
        };
        let k3 = CacheKey::PublishedPosts {
            page: 2,
            per_page: 10,
        };
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[tokio::test]
    #[serial]
    async fn post_list_cache_roundtrip() {
        let key = CacheKey::PublishedPosts {
            page: 999,
            per_page: 99,
        };
        let posts = vec![PostListItem {
            id: 1,
            author_id: 1,
            title: "List Item".to_string(),
            slug: "list-item".to_string(),
            summary: None,
            status: PostStatus::Published,
            published_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deleted_at: None,
            tags: vec!["rust".to_string()],
            cover_image: None,
            reading_time: 1,
            word_count: 10,
        }];

        set_post_list(&key, posts.clone(), 1).await;
        let cached = get_post_list(&key).await;

        assert!(cached.is_some());
        let (cached_posts, cached_total) = cached.unwrap();
        assert_eq!(cached_posts.len(), 1);
        assert_eq!(cached_posts[0].title, "List Item");
        assert_eq!(cached_total, 1);
    }

    #[tokio::test]
    #[serial]
    async fn tag_list_cache_roundtrip() {
        let tags = vec![Tag {
            id: 1,
            name: "rust".to_string(),
            post_count: 5,
        }];

        set_tag_list(tags.clone()).await;
        let cached = get_tag_list().await;

        assert!(cached.is_some());
        assert_eq!(cached.unwrap()[0].name, "rust");
    }

    #[tokio::test]
    #[serial]
    async fn single_post_cache_roundtrip() {
        let post = Some(Post {
            id: 1,
            author_id: 1,
            title: "Test".to_string(),
            slug: "test".to_string(),
            summary: None,
            content_md: "content".to_string(),
            content_html: None,
            status: PostStatus::Published,
            published_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deleted_at: None,
            tags: vec![],
            cover_image: None,
            reading_time: 1,
            word_count: 10,
            toc_html: None,
            prev_post: None,
            next_post: None,
        });

        set_post_by_slug("test", post.clone()).await;
        let cached = get_post_by_slug("test").await;

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().unwrap().title, "Test");
    }

    #[tokio::test]
    #[serial]
    async fn post_stats_cache_roundtrip() {
        let stats = PostStats {
            total: 10,
            drafts: 3,
            published: 7,
        };

        set_post_stats(stats.clone()).await;
        let cached = get_post_stats().await;

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().total, 10);
    }

    #[tokio::test]
    #[serial]
    async fn cache_invalidation_works() {
        let post = Some(Post {
            id: 42,
            author_id: 1,
            title: "Invalidation Test".to_string(),
            slug: "invalidation-test".to_string(),
            summary: None,
            content_md: "test".to_string(),
            content_html: None,
            status: PostStatus::Published,
            published_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deleted_at: None,
            tags: vec![],
            cover_image: None,
            reading_time: 1,
            word_count: 4,
            toc_html: None,
            prev_post: None,
            next_post: None,
        });

        set_post_by_slug("invalidation-test", post.clone()).await;
        let cached_before = get_post_by_slug("invalidation-test").await;
        assert!(cached_before.is_some());

        invalidate_post_by_slug("invalidation-test").await;

        let cached_after = get_post_by_slug("invalidation-test").await;
        assert!(cached_after.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn comment_cache_roundtrip() {
        let comments = vec![PublicComment {
            id: 1,
            parent_id: None,
            depth: 0,
            author_name: "Alice".to_string(),
            author_url: None,
            avatar_url: "https://example.com/avatar".to_string(),
            content_html: Some("<p>Hello</p>".to_string()),
            created_at: "刚刚".to_string(),
            created_at_iso: "2026-01-01T00:00:00Z".to_string(),
        }];

        set_comments_by_post(42, comments.clone()).await;
        let cached = get_comments_by_post(42).await;

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn pending_count_cache_roundtrip() {
        set_pending_count(7).await;
        let cached = get_pending_count().await;

        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), 7);
    }

    #[tokio::test]
    #[serial]
    async fn comment_cache_invalidation() {
        set_comments_by_post(99, vec![]).await;
        assert!(get_comments_by_post(99).await.is_some());

        invalidate_comments_by_post(99).await;
        assert!(get_comments_by_post(99).await.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn pending_count_invalidation() {
        set_pending_count(3).await;
        assert!(get_pending_count().await.is_some());

        invalidate_pending_count().await;
        assert!(get_pending_count().await.is_none());
    }

}

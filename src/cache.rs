#[cfg(feature = "server")]
use moka::future::Cache;
#[cfg(feature = "server")]
use std::sync::LazyLock;
#[cfg(feature = "server")]
use std::time::Duration;

#[cfg(feature = "server")]
use crate::models::post::{Post, PostStats, Tag};

// ============================================================================
// Cache TTL Configuration
// ============================================================================

#[cfg(feature = "server")]
const TTL_POST_LIST: Duration = Duration::from_secs(60);
#[cfg(feature = "server")]
const TTL_TAG_LIST: Duration = Duration::from_secs(300);
#[cfg(feature = "server")]
const TTL_SINGLE_POST: Duration = Duration::from_secs(600);
#[cfg(feature = "server")]
const TTL_POST_STATS: Duration = Duration::from_secs(60);
#[cfg(feature = "server")]
const TTL_TAG_POSTS: Duration = Duration::from_secs(120);

// ============================================================================
// Cache Key Types
// ============================================================================

#[cfg(feature = "server")]
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum CacheKey {
    PublishedPosts { page: i32, per_page: i32 },
    TotalPublishedPosts,
    AllTags,
    PostBySlug(String),
    PostsByTag(String),
    PostStats,
}



// ============================================================================
// Cache Instances
// ============================================================================

#[cfg(feature = "server")]
pub type PostListCache = Cache<CacheKey, (Vec<Post>, i64)>;

#[cfg(feature = "server")]
pub type TagListCache = Cache<CacheKey, Vec<Tag>>;

#[cfg(feature = "server")]
pub type SinglePostCache = Cache<CacheKey, Option<Post>>;

#[cfg(feature = "server")]
pub type PostStatsCache = Cache<CacheKey, PostStats>;

#[cfg(feature = "server")]
static POST_LIST_CACHE: LazyLock<PostListCache> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(100)
        .time_to_live(TTL_POST_LIST)
        .build()
});

#[cfg(feature = "server")]
static TAG_LIST_CACHE: LazyLock<TagListCache> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(50)
        .time_to_live(TTL_TAG_LIST)
        .build()
});

#[cfg(feature = "server")]
static SINGLE_POST_CACHE: LazyLock<SinglePostCache> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(200)
        .time_to_live(TTL_SINGLE_POST)
        .build()
});

#[cfg(feature = "server")]
static POST_STATS_CACHE: LazyLock<PostStatsCache> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(10)
        .time_to_live(TTL_POST_STATS)
        .build()
});

#[cfg(feature = "server")]
static TAG_POSTS_CACHE: LazyLock<PostListCache> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(100)
        .time_to_live(TTL_TAG_POSTS)
        .build()
});

// ============================================================================
// Public Cache API
// ============================================================================

#[cfg(feature = "server")]
pub async fn get_post_list(key: &CacheKey) -> Option<(Vec<Post>, i64)> {
    POST_LIST_CACHE.get(key).await
}

#[cfg(feature = "server")]
pub async fn set_post_list(key: &CacheKey, posts: Vec<Post>, total: i64) {
    let _ = POST_LIST_CACHE.insert(key.clone(), (posts, total)).await;
}

#[cfg(feature = "server")]
pub async fn get_total_published_posts() -> Option<i64> {
    POST_LIST_CACHE.get(&CacheKey::TotalPublishedPosts).await.map(|(_, total)| total)
}

#[cfg(feature = "server")]
pub async fn set_total_published_posts(total: i64) {
    let _ = POST_LIST_CACHE.insert(CacheKey::TotalPublishedPosts, (vec![], total)).await;
}

#[cfg(feature = "server")]
pub async fn get_tag_list() -> Option<Vec<Tag>> {
    TAG_LIST_CACHE.get(&CacheKey::AllTags).await
}

#[cfg(feature = "server")]
pub async fn set_tag_list(tags: Vec<Tag>) {
    let _ = TAG_LIST_CACHE.insert(CacheKey::AllTags, tags).await;
}

#[cfg(feature = "server")]
pub async fn get_post_by_slug(slug: &str) -> Option<Option<Post>> {
    SINGLE_POST_CACHE
        .get(&CacheKey::PostBySlug(slug.to_string()))
        .await
}

#[cfg(feature = "server")]
pub async fn set_post_by_slug(slug: &str, post: Option<Post>) {
    let _ = SINGLE_POST_CACHE
        .insert(CacheKey::PostBySlug(slug.to_string()), post)
        .await;
}

#[cfg(feature = "server")]
pub async fn get_posts_by_tag(tag: &str) -> Option<(Vec<Post>, i64)> {
    TAG_POSTS_CACHE
        .get(&CacheKey::PostsByTag(tag.to_string()))
        .await
}

#[cfg(feature = "server")]
pub async fn set_posts_by_tag(tag: &str, posts: Vec<Post>, total: i64) {
    let _ = TAG_POSTS_CACHE
        .insert(CacheKey::PostsByTag(tag.to_string()), (posts, total))
        .await;
}

#[cfg(feature = "server")]
pub async fn get_post_stats() -> Option<PostStats> {
    POST_STATS_CACHE.get(&CacheKey::PostStats).await
}

#[cfg(feature = "server")]
pub async fn set_post_stats(stats: PostStats) {
    let _ = POST_STATS_CACHE.insert(CacheKey::PostStats, stats).await;
}

// ============================================================================
// Cache Invalidation
// ============================================================================

#[cfg(feature = "server")]
pub fn invalidate_post_lists() {
    POST_LIST_CACHE.invalidate_all();
}

#[cfg(feature = "server")]
pub fn invalidate_all_tags() {
    TAG_LIST_CACHE.invalidate_all();
}

#[cfg(feature = "server")]
pub async fn invalidate_post_by_slug(slug: &str) {
    SINGLE_POST_CACHE
        .invalidate(&CacheKey::PostBySlug(slug.to_string()))
        .await;
}

#[cfg(feature = "server")]
pub async fn invalidate_posts_by_tag(tag: &str) {
    TAG_POSTS_CACHE
        .invalidate(&CacheKey::PostsByTag(tag.to_string()))
        .await;
}

#[cfg(feature = "server")]
pub fn invalidate_post_stats() {
    POST_STATS_CACHE.invalidate_all();
}

#[cfg(feature = "server")]
pub fn invalidate_all_post_caches() {
    POST_LIST_CACHE.invalidate_all();
    TAG_LIST_CACHE.invalidate_all();
    SINGLE_POST_CACHE.invalidate_all();
    POST_STATS_CACHE.invalidate_all();
    TAG_POSTS_CACHE.invalidate_all();
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::models::post::PostStatus;

    #[test]
    fn cache_key_equality() {
        let k1 = CacheKey::PublishedPosts { page: 1, per_page: 10 };
        let k2 = CacheKey::PublishedPosts { page: 1, per_page: 10 };
        let k3 = CacheKey::PublishedPosts { page: 2, per_page: 10 };
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[tokio::test]
    async fn post_list_cache_roundtrip() {
        let key = CacheKey::PublishedPosts { page: 999, per_page: 99 };
        let posts = vec![];
        
        set_post_list(&key, posts.clone(), 0).await;
        let cached = get_post_list(&key).await;
        
        assert!(cached.is_some());
        let (cached_posts, cached_total) = cached.unwrap();
        assert_eq!(cached_posts.len(), 0);
        assert_eq!(cached_total, 0);
    }

    #[tokio::test]
    async fn tag_list_cache_roundtrip() {
        let tags = vec![Tag { id: 1, name: "rust".to_string(), post_count: 5 }];
        
        set_tag_list(tags.clone()).await;
        let cached = get_tag_list().await;
        
        assert!(cached.is_some());
        assert_eq!(cached.unwrap()[0].name, "rust");
    }

    #[tokio::test]
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
    async fn post_stats_cache_roundtrip() {
        let stats = PostStats { total: 10, drafts: 3, published: 7 };
        
        set_post_stats(stats.clone()).await;
        let cached = get_post_stats().await;
        
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().total, 10);
    }

    #[tokio::test]
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
}

#[cfg(feature = "server")]
use moka::future::Cache;
#[cfg(feature = "server")]
use std::sync::LazyLock;
#[cfg(feature = "server")]
use std::time::Duration;

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

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum CacheKey {
    PublishedPosts { page: i32, per_page: i32 },
    AllTags,
    PostBySlug(String),
    PostsByTag(String),
    PostStats,
}

impl CacheKey {
    pub(crate) fn as_string(&self) -> String {
        match self {
            CacheKey::PublishedPosts { page, per_page } => {
                format!("posts:list:{}:{}", page, per_page)
            }
            CacheKey::AllTags => "tags:all".to_string(),
            CacheKey::PostBySlug(slug) => format!("post:slug:{}", slug),
            CacheKey::PostsByTag(tag) => format!("posts:tag:{}", tag),
            CacheKey::PostStats => "posts:stats".to_string(),
        }
    }
}

// ============================================================================
// Cache Instances
// ============================================================================

#[cfg(feature = "server")]
pub type PostListCache = Cache<CacheKey, Vec<Post>>;

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
pub async fn get_post_list(key: &CacheKey) -> Option<Vec<Post>> {
    POST_LIST_CACHE.get(key).await
}

#[cfg(feature = "server")]
pub async fn set_post_list(key: &CacheKey, posts: Vec<Post>) {
    let _ = POST_LIST_CACHE.insert(key.clone(), posts).await;
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
    SINGLE_POST_CACHE.get(&CacheKey::PostBySlug(slug.to_string())).await
}

#[cfg(feature = "server")]
pub async fn set_post_by_slug(slug: &str, post: Option<Post>) {
    let _ = SINGLE_POST_CACHE
        .insert(CacheKey::PostBySlug(slug.to_string()), post)
        .await;
}

#[cfg(feature = "server")]
pub async fn get_posts_by_tag(tag: &str) -> Option<Vec<Post>> {
    TAG_POSTS_CACHE
        .get(&CacheKey::PostsByTag(tag.to_string()))
        .await
}

#[cfg(feature = "server")]
pub async fn set_posts_by_tag(tag: &str, posts: Vec<Post>) {
    let _ = TAG_POSTS_CACHE
        .insert(CacheKey::PostsByTag(tag.to_string()), posts)
        .await;
}

#[cfg(feature = "server")]
pub async fn get_post_stats() -> Option<PostStats> {
    POST_STATS_CACHE.get(&CacheKey::PostStats).await
}

#[cfg(feature = "server")]
pub async fn set_post_stats(stats: PostStats) {
    let _ = POST_STATS_CACHE
        .insert(CacheKey::PostStats, stats)
        .await;
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

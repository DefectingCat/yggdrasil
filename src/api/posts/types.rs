use crate::models::post::{Post, PostStats, Tag};

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
    pub total: i64,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RebuildResult {
    pub rebuilt: u64,
    pub failed: u64,
    pub errors: Vec<String>,
}

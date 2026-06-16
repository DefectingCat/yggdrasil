//! 文章 API 的请求与响应数据结构。

use crate::models::post::{Post, PostStats, Tag};

/// 创建/更新/删除文章的统一响应结构。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreatePostResponse {
    /// 操作是否成功。
    pub success: bool,
    /// 提示信息。
    pub message: String,
    /// 新文章 id，失败时为 None。
    pub post_id: Option<i32>,
    /// 最终 slug，失败时为 None。
    pub slug: Option<String>,
}

/// 文章列表响应。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PostListResponse {
    /// 文章列表。
    pub posts: Vec<Post>,
    /// 符合查询条件的总数。
    pub total: i64,
}

/// 标签列表响应。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagListResponse {
    /// 标签列表。
    pub tags: Vec<Tag>,
}

/// 文章统计响应。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PostStatsResponse {
    /// 文章统计信息。
    pub stats: PostStats,
}

/// 单篇文章详情响应。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SinglePostResponse {
    /// 文章详情，不存在时为 None。
    pub post: Option<Post>,
}

/// Markdown 重建结果响应。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RebuildResult {
    /// 成功重建的文章数量。
    pub rebuilt: u64,
    /// 重建失败的文章数量。
    pub failed: u64,
    /// 失败信息摘要（最多 5 条）。
    pub errors: Vec<String>,
}

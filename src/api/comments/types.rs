use crate::models::comment::{AdminComment, PublicComment};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct CreateCommentRequest {
    pub post_id: i32,
    pub parent_id: Option<i64>,
    pub author_name: String,
    pub author_email: String,
    pub author_url: Option<String>,
    pub content_md: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentResponse {
    pub success: bool,
    pub message: String,
    pub error_code: Option<String>,
    #[serde(default)]
    pub comment_id: Option<i64>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub depth: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentTreeResponse {
    pub comments: Vec<PublicComment>,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentCountResponse {
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingCommentsResponse {
    pub comments: Vec<AdminComment>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllCommentsResponse {
    pub comments: Vec<AdminComment>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingCountResponse {
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStatusResponse {
    pub success: bool,
    pub updated_count: i64,
    pub message: String,
}

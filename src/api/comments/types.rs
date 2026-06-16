//! 评论 API 的请求与响应数据结构。

use crate::models::comment::{AdminComment, PublicComment};
use serde::{Deserialize, Serialize};

/// 创建/审核/删除评论的统一响应结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentResponse {
    /// 操作是否成功。
    pub success: bool,
    /// 提示信息。
    pub message: String,
    /// 错误码，成功时为 None。
    pub error_code: Option<String>,
    /// 新评论 id。
    #[serde(default)]
    pub comment_id: Option<i64>,
    /// 评论者头像 URL。
    #[serde(default)]
    pub avatar_url: Option<String>,
    /// 评论嵌套深度。
    #[serde(default)]
    pub depth: Option<i32>,
}

/// 评论树响应：包含文章下的全部已审核评论。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentTreeResponse {
    /// 评论列表。
    pub comments: Vec<PublicComment>,
    /// 评论总数。
    pub count: i64,
}

/// 评论计数响应。
///
/// 此类型仅在服务端函数体中构造；保留 `#[allow(dead_code)]` 以避免 WASM 构建中
/// 因函数体被剥离而产生的未使用警告。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct CommentCountResponse {
    /// 评论数量。
    pub count: i64,
}

/// 待审核评论列表响应。
///
/// 当前前端未直接调用 `get_pending_comments`，此类型仅在服务端函数体中构造；
/// 保留 `#[allow(dead_code)]` 以避免 WASM 构建中因函数体被剥离而产生的未使用警告。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PendingCommentsResponse {
    /// 待审核评论列表。
    pub comments: Vec<AdminComment>,
    /// 总数。
    pub total: i64,
}

/// 全部评论列表响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllCommentsResponse {
    /// 评论列表。
    pub comments: Vec<AdminComment>,
    /// 总数。
    pub total: i64,
}

/// 待审核评论计数响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingCountResponse {
    /// 待审核数量。
    pub count: i64,
}

/// 批量更新状态响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStatusResponse {
    /// 操作是否成功。
    pub success: bool,
    /// 实际更新的行数。
    pub updated_count: i64,
    /// 提示信息。
    pub message: String,
}

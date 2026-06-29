//! 数据模型模块。
//!
//! 定义博客系统使用的核心领域模型，包括文章（Post）、用户（User）与评论（Comment）。
//! 这些结构体通过 serde 在服务端与客户端之间共享序列化。

/// 评论模型及其状态枚举。
pub mod comment;
/// 文章模型、文章状态、标签与统计信息。
pub mod post;
/// 回收站与站点配置模型。
pub mod settings;
/// 用户模型、用户角色与可公开用户信息。
pub mod user;

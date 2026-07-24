//! 素材（图片）模型。
//!
//! `assets` 表是 `uploads/` 目录的元数据注册表：磁盘是字节唯一存储，
//! 本表承载路径、尺寸、alt 等管理性字段。`asset_refs` 记录文章引用关系。
//! 这些结构体通过 serde 在服务端与客户端之间共享序列化。
//!
//! id 以 String 承载（SQL 侧 `id::text` 读出、`$1::uuid` 写入），
//! 避免把 server-only 的 uuid crate 引入 WASM 前端构建。

use serde::{Deserialize, Serialize};

/// 素材记录（对应 assets 表一行）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Asset {
    pub id: String,
    /// 相对路径，如 "2026/07/24/153000.<uuid>.webp"（不含 /uploads/ 前缀）。
    pub path: String,
    pub filename: String,
    pub mime: String,
    pub size_bytes: i64,
    pub width: i32,
    pub height: i32,
    pub alt: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 引用该素材的一篇文章（素材详情/删除拦截时列出）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetRef {
    pub post_id: i32,
    pub title: String,
}

/// 列表页 DTO：素材本体 + 引用计数 + 引用文章列表。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetDto {
    #[serde(flatten)]
    pub asset: Asset,
    pub ref_count: i64,
    pub refs: Vec<AssetRef>,
}

/// 列表筛选：按引用状态。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum AssetFilter {
    #[default]
    All,
    Used,
    Orphan,
}

/// 列表排序。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum AssetSort {
    #[default]
    CreatedDesc,
    SizeDesc,
}

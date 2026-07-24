//! 素材管理 API 模块。
//!
//! 管理 `uploads/` 图片的注册表（assets 表）与引用关系（asset_refs 表）：
//! 分页列表、删除保护、孤儿清理、全量重建索引。
//! 全部为 Dioxus server function，仅 admin 可用。

/// 素材删除与孤儿清理。
pub mod delete;
/// 素材分页列表。
pub mod list;
/// 素材索引全量重建。
pub mod rebuild;
/// 请求与响应数据结构。
pub mod types;

pub use delete::{delete_asset, purge_orphan_assets, update_asset_alt};
pub use list::list_assets;
pub use rebuild::rebuild_assets_index;
pub use types::{AssetListResponse, AssetOpResponse, PurgeOrphansResponse, RebuildAssetsResponse};

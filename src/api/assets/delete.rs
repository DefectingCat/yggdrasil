//! 素材删除与孤儿清理接口。
//!
//! 删除保护：被任何文章（含回收站文章）引用的素材禁删，返回引用列表；
//! 孤儿素材硬删除（文件 + DB 行 + 派生缓存）。一键清理仅作用于
//! 无引用且超过 7 天保护窗的素材（保护未保存草稿的引用）。
//! Dioxus server function，注册在 `/api` 路径下，仅 admin 可用。

use dioxus::prelude::*;

use super::types::{AssetOpResponse, PurgeOrphansResponse};

/// 更新素材 alt（管理性备注，不回写已有文章 HTML）。
#[server(UpdateAssetAlt, "/api")]
pub async fn update_asset_alt(id: String, alt: String) -> Result<AssetOpResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::api::auth::get_current_admin_user;
        use crate::api::error::AppError;
        use crate::db::pool::get_conn;

        let _admin = get_current_admin_user().await?;
        let client = get_conn().await.map_err(AppError::db_conn)?;

        // id 在边界处从 String 解析为 Uuid（非法 id 属业务错误，不走 500）。
        let asset_uuid = match uuid::Uuid::parse_str(&id) {
            Ok(u) => u,
            Err(_) => return Ok(AssetOpResponse::err("素材 id 非法".to_string())),
        };

        let alt = alt.trim().to_string();
        let updated = client
            .execute(
                "UPDATE assets SET alt = NULLIF($2, ''), updated_at = NOW() WHERE id = $1",
                &[&asset_uuid, &alt],
            )
            .await
            .map_err(AppError::query)?;

        if updated == 0 {
            return Ok(AssetOpResponse::err("素材不存在".to_string()));
        }
        Ok(AssetOpResponse::ok("alt 已更新".to_string()))
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

/// 删除单张素材。
///
/// 引用中 → `Ok(success:false)` + 引用文章列表（业务拒绝不走 Err）；
/// 孤儿 → 物理删除文件、DB 行（refs 级联）与派生缓存。
#[server(DeleteAsset, "/api")]
pub async fn delete_asset(id: String) -> Result<AssetOpResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::api::auth::get_current_admin_user;
        use crate::api::error::AppError;
        use crate::db::pool::get_conn;
        use crate::models::asset::AssetRef;

        let _admin = get_current_admin_user().await?;
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let asset_uuid = match uuid::Uuid::parse_str(&id) {
            Ok(u) => u,
            Err(_) => return Ok(AssetOpResponse::err("素材 id 非法".to_string())),
        };

        let row = client
            .query_opt(
                "SELECT id AS id, path, filename FROM assets WHERE id = $1",
                &[&asset_uuid],
            )
            .await
            .map_err(AppError::query)?;
        let Some(row) = row else {
            return Ok(AssetOpResponse::err("素材不存在".to_string()));
        };
        let path: String = row.get("path");

        // 引用检查：含回收站文章（其 purge 时 refs 级联删，图自然变孤儿）。
        let ref_rows = client
            .query(
                "SELECT p.id, p.title FROM asset_refs r JOIN posts p ON p.id = r.post_id \
                 WHERE r.asset_id = $1 ORDER BY p.id",
                &[&asset_uuid],
            )
            .await
            .map_err(AppError::query)?;
        if !ref_rows.is_empty() {
            let refs: Vec<AssetRef> = ref_rows
                .iter()
                .map(|r| AssetRef {
                    post_id: r.get(0),
                    title: r.get(1),
                })
                .collect();
            return Ok(AssetOpResponse {
                success: false,
                message: format!("该素材正被 {} 篇文章引用，无法删除", refs.len()),
                refs,
            });
        }

        // 孤儿：先删文件（NotFound 容忍——磁盘与 DB 可能已不一致），再删 DB 行。
        let file_path = format!("uploads/{}", path);
        if let Err(e) = tokio::fs::remove_file(&file_path).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("Remove asset file failed ({}): {}", file_path, e);
            }
        }
        client
            .execute("DELETE FROM assets WHERE id = $1", &[&asset_uuid])
            .await
            .map_err(AppError::query)?;
        crate::api::image::invalidate_asset_caches(&path).await;

        Ok(AssetOpResponse::ok("已删除".to_string()))
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

/// 一键清理孤儿素材：无引用且 created_at 早于 7 天保护窗。
///
/// 逐项删文件（容忍单项失败），最后批量删 DB 行。
/// 返回清理数量、释放字节数与文件删除失败数。
#[server(PurgeOrphanAssets, "/api")]
pub async fn purge_orphan_assets() -> Result<PurgeOrphansResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::api::auth::get_current_admin_user;
        use crate::api::error::AppError;
        use crate::db::pool::get_conn;

        let _admin = get_current_admin_user().await?;
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let rows = client
            .query(
                "SELECT a.id AS id, a.path, a.size_bytes FROM assets a \
                 WHERE NOT EXISTS (SELECT 1 FROM asset_refs r WHERE r.asset_id = a.id) \
                   AND a.created_at < NOW() - make_interval(days => $1)",
                &[&super::list::PURGE_GRACE_DAYS],
            )
            .await
            .map_err(AppError::query)?;

        if rows.is_empty() {
            return Ok(PurgeOrphansResponse {
                success: true,
                message: "没有可清理的孤儿素材".to_string(),
                deleted_count: 0,
                freed_bytes: 0,
                failures: 0,
            });
        }

        let mut ids: Vec<uuid::Uuid> = Vec::with_capacity(rows.len());
        let mut freed_bytes: i64 = 0;
        let mut failures: i64 = 0;
        for row in &rows {
            let id: uuid::Uuid = row.get("id");
            let path: String = row.get("path");
            freed_bytes += row.get::<_, i64>("size_bytes");
            let file_path = format!("uploads/{}", path);
            if let Err(e) = tokio::fs::remove_file(&file_path).await {
                // NotFound 可容忍（文件可能已被手动删）；其他错误计入 failures，
                // DB 行照删——残留文件由重建索引的反向语义兜底。
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!("Purge: remove file failed ({}): {}", file_path, e);
                    failures += 1;
                }
            }
            crate::api::image::invalidate_asset_caches(&path).await;
            ids.push(id);
        }

        let deleted = client
            .execute("DELETE FROM assets WHERE id = ANY($1)", &[&ids])
            .await
            .map_err(AppError::query)?;

        Ok(PurgeOrphansResponse {
            success: true,
            message: format!("已清理 {} 张孤儿素材", deleted),
            deleted_count: deleted as i64,
            freed_bytes,
            failures,
        })
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

//! 站点设置 MCP 工具：读取/更新回收站自动清理配置。
//!
//! 与 `src/api/settings.rs` 的 `get_trash_settings` / `update_trash_settings`
//! server-fn 一致（同样的 SQL、同样的 clamp）。区别仅在鉴权入口：web 走 cookie
//! `get_current_admin_user()`，MCP 走 bearer token → `McpPrincipal`，要求 admin 作用域。
//!
//! 缓存失效：与 web fn 保持一致——`update_trash_settings` **不做任何缓存失效**。
//! 理由：回收站配置只影响管理后台（SSR 缓存在 `admin/`，`invalidate_ssr_all_public`
//! 明确保留不动）和后台清理任务，没有公开页缓存表面，故无需失效。
//! （约束 #5 要求「按 web admin server fn 的方式失效」——该 fn 的方式就是不失效。）

#![cfg(feature = "server")]

use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, TextContent};
use rmcp::{schemars, tool, tool_router, ErrorData as McpError};

use super::common::require_admin;
use serde::Deserialize;

use crate::api::error::AppError;
use crate::db::pool::get_conn;
use crate::models::settings::{TrashSettings, DEFAULT_AUTO_PURGE_ENABLED, DEFAULT_RETENTION_DAYS};

/// `get_settings` 入参（无字段——预留扩展点，未来可按子域过滤）。
#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct GetSettingsParams {}

/// `update_settings` 入参：两项回收站配置。
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateSettingsParams {
    /// 是否启用回收站自动清理。
    pub auto_purge_enabled: bool,
    /// 已删除文章保留天数（会被 clamp 到 [1, 365]）。
    pub retention_days: i32,
}

#[tool_router(router = settings_router, vis = "pub")]
impl crate::mcp::server::YggMcpServer {
    /// 读取站点回收站设置。要求 admin 作用域。
    #[tool(description = "读取站点回收站配置（自动清理开关 + 保留天数）。需要 admin 作用域。")]
    async fn get_settings(
        &self,
        Parameters(_p): Parameters<GetSettingsParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        require_admin(&parts, "get_settings")?;

        let settings = load_trash_settings()
            .await
            .map_err(|e| McpError::internal_error(format!("settings read failed: {e:?}"), None))?;

        let text = serde_json::to_string_pretty(&settings)
            .map_err(|e| McpError::internal_error(format!("encode failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::Text(
            TextContent::new(text),
        )]))
    }

    /// 更新站点回收站设置。要求 admin 作用域。retention_days 会 clamp 到 [1, 365]。
    #[tool(
        description = "更新站点回收站配置（自动清理开关 + 保留天数）。retention_days 会被钳制到 1..=365。需要 admin 作用域。"
    )]
    async fn update_settings(
        &self,
        Parameters(UpdateSettingsParams {
            auto_purge_enabled,
            retention_days,
        }): Parameters<UpdateSettingsParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        require_admin(&parts, "update_settings")?;

        let retention_days = TrashSettings::clamp_retention(retention_days);

        let updated = save_trash_settings(auto_purge_enabled, retention_days)
            .await
            .map_err(|e| McpError::internal_error(format!("settings write failed: {e:?}"), None))?;

        let text = serde_json::to_string_pretty(&updated)
            .map_err(|e| McpError::internal_error(format!("encode failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::Text(
            TextContent::new(text),
        )]))
    }
}

/// 读取回收站配置（与 `get_trash_settings` 的 SQL 一致）。
///
/// settings 表缺失键时回退默认值，保证向后兼容。
async fn load_trash_settings() -> Result<TrashSettings, AppError> {
    let client = get_conn().await.map_err(AppError::db_conn)?;

    let enabled: bool = client
        .query_opt(
            "SELECT value FROM settings WHERE key = 'trash_auto_purge_enabled'",
            &[],
        )
        .await
        .map_err(AppError::query)?
        .and_then(|r| r.get::<_, String>("value").parse().ok())
        .unwrap_or(DEFAULT_AUTO_PURGE_ENABLED);

    let days: i32 = client
        .query_opt(
            "SELECT value FROM settings WHERE key = 'trash_retention_days'",
            &[],
        )
        .await
        .map_err(AppError::query)?
        .and_then(|r| r.get::<_, String>("value").parse().ok())
        .unwrap_or(DEFAULT_RETENTION_DAYS);

    Ok(TrashSettings {
        auto_purge_enabled: enabled,
        retention_days: TrashSettings::clamp_retention(days),
    })
}

/// 写入回收站配置（与 `update_trash_settings` 的 UPSERT 一致）。返回写入后的值。
async fn save_trash_settings(
    auto_purge_enabled: bool,
    retention_days: i32,
) -> Result<TrashSettings, AppError> {
    let client = get_conn().await.map_err(AppError::db_conn)?;

    client
        .execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('trash_auto_purge_enabled', $1, NOW())
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()",
            &[&auto_purge_enabled.to_string()],
        )
        .await
        .map_err(AppError::query)?;

    client
        .execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('trash_retention_days', $1, NOW())
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()",
            &[&retention_days.to_string()],
        )
        .await
        .map_err(AppError::query)?;

    tracing::info!(
        "MCP: trash settings updated: auto_purge={}, retention_days={}",
        auto_purge_enabled,
        retention_days
    );

    Ok(TrashSettings {
        auto_purge_enabled,
        retention_days,
    })
}

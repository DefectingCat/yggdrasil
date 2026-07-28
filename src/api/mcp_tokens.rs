//! MCP 访问令牌管理：Dioxus server functions。
//!
//! 管理员在后台 `/admin/mcp` 签发/查看/撤销为 AI 客户端（Claude Code / Cursor /
//! Cline）准备的 bearer 令牌。明文 token 仅在签发与「重新查看」时返回给管理员，
//! 数据库只存 AES-GCM 密文（`token_enc`，可解密重查）+ SHA-256 哈希（`token_hash`，
//! 每请求 O(1) 常量查找，见 `src/mcp/auth.rs`）。
//!
//! 鉴权走 cookie session（`get_current_admin_user`），与其它后台 server-fn 一致；
//! MCP 工具路径（bearer）无法调用这些 server-fn——那是 `src/mcp/tools/*` 的职责。

use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::models::mcp_token::McpToken;
use crate::models::mcp_token::{CreateTokenResponse, McpTokenSummary, TokenScope};

/// 令牌有效期预设：管理员在 UI 上从下拉菜单选择。
///
/// 序列化形式供前端选择回传（`days1` / `days7` / `days30` / `days90` / `never`）。
/// `Never` 对应 `expires_at = NULL`（长期令牌）；其余按当前时间 + N 天计算。
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TokenLifetime {
    /// 1 天（默认推荐：最小权限、轮换友好）。
    Days1,
    /// 7 天。
    Days7,
    /// 30 天。
    Days30,
    /// 90 天。
    Days90,
    /// 永不过期（`expires_at = NULL`）。仅用于可信长期客户端。
    Never,
}

impl TokenLifetime {
    /// 计算签发时刻对应的过期时间戳（UTC）。`Never` 返回 `None`。
    #[cfg(feature = "server")]
    fn expires_at(self) -> Option<chrono::DateTime<chrono::Utc>> {
        let now = chrono::Utc::now();
        match self {
            TokenLifetime::Days1 => Some(now + chrono::Duration::days(1)),
            TokenLifetime::Days7 => Some(now + chrono::Duration::days(7)),
            TokenLifetime::Days30 => Some(now + chrono::Duration::days(30)),
            TokenLifetime::Days90 => Some(now + chrono::Duration::days(90)),
            TokenLifetime::Never => None,
        }
    }
}

/// 签发新的 MCP 令牌。
///
/// 生成明文 `ygg_<32 hex>`，AES-GCM 加密后存密文 + SHA-256 哈希；明文随响应一次性
/// 返回给管理员（后续可经 [`reveal_mcp_token`] 重新查看）。仅 admin。
#[server]
pub async fn create_mcp_token(
    name: String,
    scope: TokenScope,
    lifetime: TokenLifetime,
) -> Result<CreateTokenResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::api::auth::get_current_admin_user;
        use crate::api::error::AppError;
        use crate::db::pool::get_conn;
        use crate::mcp::auth::{hash_token, TOKEN_PREFIX};
        use crate::mcp::crypto::encrypt_token;

        let admin = get_current_admin_user().await?;

        // 名称规范化与校验：去空白后非空，限制长度。
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::BadRequest("令牌名称不能为空".to_string()).into());
        }
        if name.chars().count() > 64 {
            return Err(AppError::BadRequest("令牌名称过长（上限 64 字符）".to_string()).into());
        }

        // 加密主密钥必须已配置，否则无法安全存储明文。
        if crate::mcp::crypto::mcp_enc_key().is_none() {
            return Err(AppError::Internal("MCP_TOKEN_ENC_KEY 未设置").into());
        }

        // 明文 token：`ygg_` + 32 字节随机数 hex（64 hex 字符）。
        let mut bytes = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        let plaintext = format!("{TOKEN_PREFIX}{}", hex::encode(bytes));
        let hash = hash_token(&plaintext);
        let enc = encrypt_token(&plaintext)
            .ok_or(AppError::Internal("MCP_TOKEN_ENC_KEY 未设置"))?;
        let id = uuid::Uuid::new_v4();
        let expires_at = lifetime.expires_at();
        let scope_str = scope.as_str();

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let row = client
            .query_one(
                "INSERT INTO mcp_tokens \
                    (id, user_id, name, scope, token_enc, token_hash, expires_at) \
                 VALUES ($1::uuid, $2, $3, $4, $5, $6, $7) \
                 RETURNING id::text, user_id, name, scope, created_at, expires_at, \
                           last_used_at, revoked_at",
                &[
                    &id,
                    &admin.id,
                    &name,
                    &scope_str,
                    &enc,
                    &hash,
                    &expires_at,
                ],
            )
            .await
            .map_err(AppError::query)?;

        let token = row_to_mcp_token_meta(&row);
        Ok(CreateTokenResponse {
            summary: token.into(),
            plaintext,
        })
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

/// 列出当前管理员名下的全部令牌（不含任何密钥材料，仅展示用元数据）。
///
/// 按 `created_at DESC` 排序，最近签发的在前。仅 admin。
#[server]
pub async fn list_mcp_tokens() -> Result<Vec<McpTokenSummary>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::api::auth::get_current_admin_user;
        use crate::api::error::AppError;
        use crate::db::pool::get_conn;

        let admin = get_current_admin_user().await?;
        let client = get_conn().await.map_err(AppError::db_conn)?;

        let rows = client
            .query(
                "SELECT id::text, user_id, name, scope, created_at, expires_at, \
                        last_used_at, revoked_at \
                 FROM mcp_tokens \
                 WHERE user_id = $1 \
                 ORDER BY created_at DESC",
                &[&admin.id],
            )
            .await
            .map_err(AppError::query)?;

        Ok(rows
            .iter()
            .map(row_to_mcp_token_meta)
            .map(McpTokenSummary::from)
            .collect())
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

/// 重新查看令牌明文（可多次调用：明文以密文形式落库，可解密还原）。
///
/// 找不到令牌、或令牌不属于当前管理员 → 返回 `None`（不区分原因，避免探测）。
/// 仅 admin。
#[server]
pub async fn reveal_mcp_token(id: String) -> Result<Option<String>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::api::auth::get_current_admin_user;
        use crate::api::error::AppError;
        use crate::db::pool::get_conn;
        use crate::mcp::crypto::decrypt_token;

        let admin = get_current_admin_user().await?;
        let client = get_conn().await.map_err(AppError::db_conn)?;

        // id 由前端以字符串传入（表 id 列是 uuid）：解析失败视作令牌不存在。
        let id = match uuid::Uuid::parse_str(&id) {
            Ok(u) => u,
            Err(_) => return Ok(None),
        };

        // 仅取属于当前管理员的令牌的密文，避免越权解密他人令牌。
        let row = client
            .query_opt(
                "SELECT token_enc FROM mcp_tokens WHERE id = $1::uuid AND user_id = $2",
                &[&id, &admin.id],
            )
            .await
            .map_err(AppError::query)?;

        // 解密失败（密钥缺失/密文被篡改）也归一到 None：调用方无法区分，
        // 按「该令牌不可解密」处理（等同于失效）。
        Ok(row
            .map(|r| r.get::<_, String>("token_enc"))
            .and_then(|enc| decrypt_token(&enc)))
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

/// 撤销令牌（软删除：置 `revoked_at = now()`，行保留以备审计）。
///
/// 找不到或非本人令牌 → 静默无操作（不报错，避免探测）。仅 admin。
#[server]
pub async fn revoke_mcp_token(id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::api::auth::get_current_admin_user;
        use crate::api::error::AppError;
        use crate::db::pool::get_conn;

        let admin = get_current_admin_user().await?;
        let client = get_conn().await.map_err(AppError::db_conn)?;

        // id 解析失败视作令牌不存在（静默无操作，避免探测）。
        let id = match uuid::Uuid::parse_str(&id) {
            Ok(u) => u,
            Err(_) => return Ok(()),
        };

        client
            .execute(
                "UPDATE mcp_tokens SET revoked_at = NOW() \
                 WHERE id = $1::uuid AND user_id = $2 AND revoked_at IS NULL",
                &[&id, &admin.id],
            )
            .await
            .map_err(AppError::query)?;

        Ok(())
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

/// 把数据库行解析为令牌元数据（不含明文；密文/哈希已 `#[serde(skip)]`，这里置空）。
///
/// `scope` 列存的是字符串；非法值（理论不可能，除非手工改库）按 read 兜底并记日志，
/// 不 panic。
#[cfg(feature = "server")]
fn row_to_mcp_token_meta(row: &tokio_postgres::Row) -> McpToken {
    let scope_str: String = row.get("scope");
    let scope = TokenScope::from_db(&scope_str).unwrap_or_else(|| {
        tracing::warn!(scope = %scope_str, "mcp_tokens.scope 非法值，兜底为 read");
        TokenScope::Read
    });
    McpToken {
        id: row.get("id"),
        user_id: row.get("user_id"),
        name: row.get("name"),
        scope,
        token_enc: String::new(),
        token_hash: String::new(),
        created_at: row.get("created_at"),
        expires_at: row.get("expires_at"),
        last_used_at: row.get("last_used_at"),
        revoked_at: row.get("revoked_at"),
    }
}

/// 4 种客户端配置 + CLI 一行命令的序列化 DTO。
///
/// 由 `get_mcp_client_configs` server fn 返回。`ClientConfigs`（在 `src/mcp/config.rs`）
/// 是 server-only（`mcp` 模块整体 `#[cfg(feature = "server")]` 门控），这里复制字段
/// 为可两端共享的 DTO，让 WASM 前端能经 server fn 拿到配置字符串。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpClientConfigs {
    /// Claude Code（`.mcp.json` / `~/.claude.json`）与 Cursor（`~/.cursor/mcp.json`）。
    pub claude_code_json: String,
    /// Cursor 专用变体（与 claude_code_json 相同，单独列出便于标注）。
    pub cursor_json: String,
    /// Cline（`cline_mcp_settings.json`）。
    pub cline_json: String,
    /// Oh-My-Pi（`~/.pi/agent/mcp.json` / `.pi/mcp.json`）。字段名 `transport`，非 `type`。
    pub omp_json: String,
    /// 通用原始 JSON（单 server entry）。
    pub generic_json: String,
    /// Claude Code CLI 一行命令。
    pub claude_cli: String,
}

/// 根据明文令牌生成 4 种客户端配置 + CLI 一行命令。
///
/// 配置生成在服务端完成（`crate::mcp::config` 是 server-only 模块），返回给前端展示。
/// `APP_BASE_URL` 环境变量也只在服务端读取。仅 admin。
#[server]
pub async fn get_mcp_client_configs(token: String) -> Result<McpClientConfigs, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::api::auth::get_current_admin_user;

        let _admin = get_current_admin_user().await?;
        let configs = crate::mcp::config::generate_client_configs(
            &crate::mcp::config::base_url_from_env(),
            &token,
        );
        Ok(McpClientConfigs {
            claude_code_json: configs.claude_code_json,
            cursor_json: configs.cursor_json,
            cline_json: configs.cline_json,
            omp_json: configs.omp_json,
            generic_json: configs.generic_json,
            claude_cli: configs.claude_cli,
        })
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn lifetime_expires_at_days() {
        let now = chrono::Utc::now();
        let d1 = TokenLifetime::Days1.expires_at().unwrap();
        let d7 = TokenLifetime::Days7.expires_at().unwrap();
        assert!(d1 > now);
        assert!(d7 > d1);
        // 7 天与 1 天的差应≈6 天（容忍微量时钟漂移）。
        let delta = (d7 - d1).num_seconds() as f64 / 86400.0;
        assert!((5.9..6.1).contains(&delta));
    }

    #[test]
    fn lifetime_never_is_none() {
        assert!(TokenLifetime::Never.expires_at().is_none());
    }

    #[test]
    fn lifetime_serde_roundtrip() {
        let json = serde_json::to_string(&TokenLifetime::Days30).unwrap();
        assert_eq!(json, "\"days30\"");
        let back: TokenLifetime = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TokenLifetime::Days30);
    }
}

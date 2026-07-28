//! MCP 请求鉴权：bearer token → 已认证主体。
//!
//! 流程（每请求）：
//! 1. axum `from_fn` 中间件 [`mcp_auth_middleware`] 解析 `Authorization: Bearer ygg_...`；
//! 2. SHA-256 哈希后在 `mcp_tokens` 表常量查找未撤销、未过期的行；
//! 3. 命中则把 [`McpPrincipal`] { user_id, scope, token_id } 注入 request.extensions，
//!    并异步刷新 `last_used_at`；
//! 4. 未命中/缺失 → 401（不区分原因，避免探测）。
//!
//! Origin→403、协议版本头、体积上限由 rmcp 的 `StreamableHttpServerConfig` 内置，
//! 本模块只管 bearer 鉴权。`McpPrincipal` 经 rmcp 的 `Extension<http::request::Parts>`
//! 提取器在工具内读取（见 `server.rs`）。

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use sha2::{Digest, Sha256};

use crate::db::pool::get_conn;
use crate::models::mcp_token::TokenScope;

/// Bearer token 前缀（明文形式以 `ygg_` 起头，便于人眼识别与日志脱敏）。
pub const TOKEN_PREFIX: &str = "ygg_";

/// 已认证主体：由中间件注入 request.extensions，工具经 Extension 提取器读取。
#[derive(Clone, Debug)]
pub struct McpPrincipal {
    pub user_id: i32,
    pub scope: TokenScope,
    /// 令牌 DB id（String，与 model 一致；用于审计/last_used_at 刷新）。
    pub token_id: String,
}

/// 从 Authorization 头解析 bearer 明文（去前缀后返回完整 token 字符串）。
/// 非 bearer、缺前缀、解码失败统一返回 None。
pub fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    let scheme = raw.strip_prefix("Bearer ")?.trim();
    if scheme.starts_with(TOKEN_PREFIX) {
        Some(scheme.to_string())
    } else {
        None
    }
}

/// 明文 token → SHA-256 hex（与 mcp_tokens.token_hash 列一致，用于 DB 查找）。
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// MCP 鉴权中间件：无 bearer 或 token 无效 → 401；否则注入 McpPrincipal。
///
/// 注意：这是 T1 的最小实现——每请求同步查库 + 同步更新 last_used_at。
/// T6 会把 last_used_at 刷新改为节流（批量/惰性）以减负，鉴权查询本身保持同步
/// （这是认证的必要代价，无法乐观）。
pub async fn mcp_auth_middleware(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let token = match extract_bearer(req.headers()) {
        Some(t) => t,
        None => return Err(StatusCode::UNAUTHORIZED),
    };
    match resolve_principal(&token).await {
        Some(principal) => {
            req.extensions_mut().insert(principal);
            Ok(next.run(req).await)
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

/// 查库解析 token → 主体；未撤销、未过期才返回 Some。
async fn resolve_principal(token: &str) -> Option<McpPrincipal> {
    let hash = hash_token(token);
    let client = get_conn().await.ok()?;
    // 一次查询取出 + 校验所有条件；row-level 过滤避免 TOCTOU。
    let row = client
        .query_opt(
            "SELECT id, user_id, scope, expires_at, revoked_at
             FROM mcp_tokens
             WHERE token_hash = $1
               AND revoked_at IS NULL
               AND (expires_at IS NULL OR expires_at > NOW())",
            &[&hash],
        )
        .await
        .ok()??;

    let token_id: uuid::Uuid = row.get(0);
    let user_id: i32 = row.get(1);
    let scope_str: &str = row.get(2);
    let scope = TokenScope::from_db(scope_str)?;

    // best-effort 刷新 last_used_at：失败不影响鉴权（已在 Some 分支）。
    let _ = client
        .execute(
            "UPDATE mcp_tokens SET last_used_at = NOW() WHERE id = $1",
            &[&token_id],
        )
        .await;

    Some(McpPrincipal {
        user_id,
        scope,
        token_id: token_id.to_string(),
    })
}

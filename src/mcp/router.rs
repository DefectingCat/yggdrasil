//! 构建并返回挂载于 `/mcp` 的 axum 子路由。
//!
//! 装配：
//! - rmcp `StreamableHttpService`（无状态、JSON 响应、Origin 白名单）；
//! - 本 crate 的 `mcp_auth_middleware`（bearer → McpPrincipal，注入 extensions）。
//!
//! Origin 白名单：优先 `APP_BASE_URL`；否则放空（rmcp 默认对缺 Origin 放行）。
//! 生产部署 MUST 设置 `APP_BASE_URL`（见 docs/DEPLOYMENT.md），白名单才会生效。
//! 协议版本头、体积上限（4MiB 默认）由 rmcp 内置，无需此处重复。

use axum::middleware::from_fn;
use axum::Router;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};

use crate::mcp::auth::mcp_auth_middleware;
use crate::mcp::server::YggMcpServer;

/// 构建 `/mcp` 子路由（供 main.rs 的 `.merge(mcp_route)` 合并）。
///
/// 仅在 server feature 下有意义；WASM 构建不会调用本函数。
pub fn mcp_route() -> Router {
    // 计算 Origin 白名单：APP_BASE_URL 规范化后加入；未设置则空（rmcp 对缺 Origin 放行）。
    let allowed_origins: Vec<String> = std::env::var("APP_BASE_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .into_iter()
        .collect();

    let mut config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false) // 无状态（SEP-2567）
        .with_json_response(true); // 简单工具用 application/json 直回
    if !allowed_origins.is_empty() {
        config = config.with_allowed_origins(allowed_origins.iter().map(|s| s.as_str()));
    }

    let service = StreamableHttpService::new(
        || Ok::<_, std::io::Error>(YggMcpServer),
        LocalSessionManager::default().into(),
        config,
    );

    Router::new()
        .nest_service("/mcp", service)
        .layer(from_fn(mcp_auth_middleware))
}

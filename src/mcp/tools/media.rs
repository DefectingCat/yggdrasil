//! MCP 写作用域工具：媒体上传（URL 抓取）。
//!
//! Option B 的第三通道：LLM 工具只收 `url: String`（JSON-RPC 纯文本），服务端
//! 按 SSRF 防护抓取二进制，再走 [`crate::api::upload::process_image_upload`] 共享
//! 入库流水线。**二进制从不进 JSON-RPC**——彻底绕开 rmcp 4MiB 请求体上限与
//! base64 的 33% 膨胀 + 上下文窗口烧灼。
//!
//! 另有第二通道 `POST /api/mcp/upload`（bearer multipart）供 host/shell 直接 POST
//! 二进制（如 Claude Code 的 Bash+curl）；两条通道共用同一入库流水线。
//!
//! SSRF 防护（多层纵深）见 [`crate::api::url_fetch`]：强制 https、解析即锁 IP
//! 杜绝 DNS rebinding、禁重定向、流式体积上限、超时。
//!
//! 本模块仅 `feature = "server"` 编译。

#![cfg(feature = "server")]

use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router, ErrorData as McpError};
use serde::Deserialize;

use super::common::{internal, ok_json, require_scope};
use crate::models::mcp_token::TokenScope;

#[tool_router(router = media_router, vis = "pub")]
impl crate::mcp::server::YggMcpServer {
    /// 从一个图片 URL 抓取并入库（服务端转 WebP 若更小），返回可直接嵌入
    /// Markdown 正文的 `/uploads/...` URL。要求 write 作用域。
    ///
    /// 仅接受 `https://` URL；服务端做 SSRF 防护（私网/回环/保留段拒绝、
    /// DNS 锁定防 rebinding、禁重定向、体积上限）。二进制不经 JSON-RPC。
    #[tool(
        description = "从图片 URL 抓取并入库（服务端转 WebP 若更小），返回 /uploads/... URL（可直接用于 Markdown 正文 img）。仅接受 https:// URL，支持 JPEG/PNG/GIF/WebP。二进制不经 JSON-RPC。"
    )]
    async fn upload_media(
        &self,
        Parameters(p): Parameters<UploadMediaParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let _principal = require_scope(&parts, "upload_media", TokenScope::Write)?;

        // SSRF 防护抓取 + 共享入库流水线。
        let outcome = crate::api::url_fetch::fetch_and_ingest(&p.url)
            .await
            .map_err(|e| match e {
                crate::api::url_fetch::FetchError::Invalid(msg)
                | crate::api::url_fetch::FetchError::BadStatus(msg) => {
                    McpError::invalid_request(msg, None)
                }
                crate::api::url_fetch::FetchError::TooLarge => McpError::invalid_request(
                    format!(
                        "文件超过大小限制（{} bytes）",
                        crate::utils::server::MAX_FILE_SIZE
                    ),
                    None,
                ),
                crate::api::url_fetch::FetchError::Fetch(ctx) => internal(ctx, "url fetch"),
            })?;

        tracing::info!(
            "MCP media uploaded via URL: {} ({}x{}, reused={})",
            outcome.url,
            outcome.width,
            outcome.height,
            outcome.reused
        );

        ok_json(UploadResult {
            success: true,
            url: outcome.url,
            reused: outcome.reused,
            width: outcome.width,
            height: outcome.height,
            mime: outcome.mime,
        })
    }
}

// ---------------------------------------------------------------------------
// 参数与输出结构
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UploadMediaParams {
    /// 图片的 https URL（服务端抓取，二进制不经 JSON-RPC）。
    pub url: String,
    /// 替代文本（alt），目前未持久化，保留供未来扩展。
    #[serde(default)]
    #[allow(dead_code)] // 面向未来：客户端可传入，assets 表未存 alt 列
    pub alt: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct UploadResult {
    success: bool,
    url: String,
    reused: bool,
    width: u32,
    height: u32,
    mime: String,
}

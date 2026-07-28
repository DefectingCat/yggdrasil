//! MCP（Model Context Protocol）服务器。
//!
//! 把博客暴露为一个 MCP 服务器，管理员的 AI 客户端（Claude Code / Cursor / Cline）
//! 通过 `POST /mcp`（Streamable HTTP，无状态）以 bearer token 认证，可：
//! - 查询已发布文章作为知识库（read 作用域）；
//! - 执行几乎所有后台操作：文章 CRUD、评论、标签、媒体、设置、代码运行器
//!   （write / admin 作用域）。
//!
//! 传输：官方 `rmcp` crate 的 `StreamableHttpService`，挂载于 axum `/mcp`。
//! 认证：每请求由 axum 中间件解析 `Authorization: Bearer ygg_...` → 注入
//! `McpPrincipal` 到 request extensions；工具经 `Extension<http::request::Parts>`
//! 提取器读取，按作用域鉴权。Origin→403 / 协议版本头 / 体积上限由 rmcp 内置。
//!
//! 仅 server feature 编译；WASM 前端构建不含本模块（无 `#[cfg(not)]` stub 需求，
//! 整个模块用 `#[cfg(feature = "server")]` 门控，前端侧不引用任何 mcp 符号）。

#[cfg(feature = "server")]
pub mod auth;
pub mod config;
#[cfg(feature = "server")]
pub mod crypto;
#[cfg(feature = "server")]
pub mod router;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "server")]
pub mod tools;

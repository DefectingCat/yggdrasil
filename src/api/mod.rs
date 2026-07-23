//! API 层根模块。
//!
//! 按职责划分子模块，包含两类接口：
//! - Dioxus server function（`#[server(Name, "/api")]`），如 `auth`、`posts`；
//! - Axum 手动路由处理器，如 `upload`、`image`。

/// 认证相关的 Dioxus server function。
pub mod auth;
/// 代码运行接口与数据结构。
pub mod code_runner;
/// 评论相关接口。
pub mod comments;
/// CSRF 防护中间件。
pub mod csrf;
/// 数据库管理接口（运行状态 / SQL 控制台 / 导出 / 备份恢复）。
pub mod database;
/// 应用错误类型与转换。
pub mod error;
/// 健康检查端点（liveness / readiness）。
pub mod health;
/// 图片服务的 Axum 处理器。
pub mod image;
/// KaTeX 服务端数学公式渲染（server-only）。
#[cfg(feature = "server")]
pub mod katex;
/// mhchem 化学公式转译器（\ce/\pu → LaTeX，server-only）。
#[cfg(feature = "server")]
pub mod mhchem;
/// Markdown 渲染与 HTML 清理。
pub mod markdown;
/// 文章 CRUD 相关接口。
pub mod posts;
/// 限流工具。
pub mod rate_limit;
/// HTML 消毒器。
pub mod sanitizer;
/// 回收站与站点配置接口。
pub mod settings;
/// URL slug 生成与校验。
pub mod slug;
/// 图片上传的 Axum 处理器。
pub mod upload;

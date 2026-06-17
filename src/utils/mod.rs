//! 通用工具函数子模块。
//!
//! `text` 模块仅在 `server` feature 启用时编译；`time` 模块同时提供 WASM 与原生异步版本。

/// Markdown / 纯文本处理工具。
#[cfg(feature = "server")]
pub mod text;
/// 跨平台的异步睡眠等时间工具。
pub mod time;

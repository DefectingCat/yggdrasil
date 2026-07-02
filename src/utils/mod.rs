//! 通用工具函数子模块。
//!
//! - `html`：HTML 转义（两端通用）。
//! - `text`：Markdown / 纯文本处理（仅 `server` feature）。
//! - `time`：跨平台时间/睡眠工具（WASM 与原生异步版本）。

/// HTML 转义工具（前端后端通用）。
pub mod html;
/// Markdown / 纯文本处理工具。
#[cfg(feature = "server")]
pub mod text;
/// 跨平台时间/睡眠工具。
pub mod time;

//! 后台任务调度入口。
//!
//! 所有任务仅在 `server` feature 启用时编译，运行在服务端独立的 tokio 任务中。

/// 定时清理评论过期的 IP 与用户代理信息，满足隐私保护要求。
#[cfg(feature = "server")]
pub mod ip_purge;
/// 定时删除已过期会话，避免 `sessions` 表无限增长。
#[cfg(feature = "server")]
pub mod session_cleanup;

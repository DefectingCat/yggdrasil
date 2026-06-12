//! 共享的 Dioxus Hooks 模块。
//!
//! 该模块集中管理可在组件树中复用的自定义 Hook，包括：
//! - 评论草稿在浏览器 localStorage 中的持久化（WASM 端）
//! - 骨架屏延迟加载状态

/// 评论草稿持久化 Hook，基于浏览器的 localStorage（仅在 WASM 端有效）。
pub mod comment_storage;

/// 骨架屏延迟加载状态 Hook。
pub mod delayed_loading;

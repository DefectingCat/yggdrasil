//! 管理后台页面模块。
//!
//! 汇总并重新导出后台管理相关的页面组件，供路由与其他模块使用。

/// 评论管理页面模块。
pub mod comments;
/// 管理后台仪表盘页面模块。
pub mod dashboard;
/// 文章管理列表页面模块。
pub mod posts;
/// 文章编辑器页面模块（基于 Tiptap 富文本编辑器）。
pub mod write;

/// 评论管理入口组件（带默认分页）。
pub use comments::{AdminComments, AdminCommentsPage};
/// 管理后台仪表盘组件。
pub use dashboard::Admin;
/// 文章管理列表组件（带默认分页）。
pub use posts::{Posts, PostsPage};
/// 文章编辑器组件（新建与编辑模式）。
pub use write::{Write, WriteEdit};

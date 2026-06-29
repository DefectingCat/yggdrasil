//! 管理后台页面模块。
//!
//! 汇总并重新导出后台管理相关的页面组件，供路由与其他模块使用。

/// 评论管理页面模块。
pub mod comments;
/// 管理后台仪表盘页面模块。
pub mod dashboard;
/// 文章管理列表页面模块。
pub mod posts;
/// 系统管理页面模块（数据库 + 服务器状态 + SQL 控制台 + 导出 + 备份）。
pub mod system;
/// 回收站管理页面模块。
pub mod trash;
/// 文章编辑器页面模块（基于 Tiptap 富文本编辑器）。
pub mod write;

/// 评论管理入口组件（带默认分页）。
pub use comments::{AdminComments, AdminCommentsPage};
/// 管理后台仪表盘组件。
pub use dashboard::Admin;
/// 文章管理列表组件（带默认分页）。
pub use posts::{Posts, PostsPage};
/// 系统管理入口组件。
pub use system::System;
/// 回收站管理组件（带默认分页）。
pub use trash::{Trash, TrashPage};
/// 文章编辑器组件（新建与编辑模式）。
pub use write::{Write, WriteEdit};

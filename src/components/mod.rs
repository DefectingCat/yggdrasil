//! 组件模块
//!
//! 提供 Dioxus UI 组件，供前端页面（`src/pages/`）使用。
//! 包括布局（`frontend_layout`、`admin_layout`）、导航（`header`、`nav`、`footer`）、
//! 文章展示（`post`、`post_card`）、评论（`comments`）、骨架屏（`skeletons`）、
//! 表单控件（`forms`）以及图片查看器（`image_viewer`）等共享组件。

/// 后台布局组件。
pub mod admin_layout;
/// 后台页面骨架屏组件。
pub mod admin_skeleton;
/// 评论相关组件。
pub mod comments;
/// 页脚组件。
pub mod footer;
/// 表单控件组件。
pub mod forms;
/// 前台布局组件。
pub mod frontend_layout;
/// 顶部导航栏组件。
pub mod header;
/// 图片查看器组件。
pub mod image_viewer;
/// 导航组件。
pub mod nav;
/// 文章详情组件。
pub mod post;
/// 文章卡片组件。
pub mod post_card;
/// 骨架屏组件集合。
pub mod skeletons;
/// 编辑器页面骨架屏组件。
pub mod write_skeleton;

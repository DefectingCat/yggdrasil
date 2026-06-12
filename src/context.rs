//! 前端全局上下文定义。
//!
//! 当前保存当前登录用户的信息，供 Dioxus 组件在客户端与服务端渲染期间共享访问。

use dioxus::prelude::*;
use std::sync::Arc;

use crate::models::user::PublicUser;

/// 用户上下文，用于在组件树中传递登录状态。
#[derive(Clone, Copy)]
pub struct UserContext {
    /// 当前登录用户，未登录时为 `None`。
    pub user: Signal<Option<Arc<PublicUser>>>,
    /// 是否已完成会话校验，避免重复触发验证请求。
    pub checked: Signal<bool>,
}

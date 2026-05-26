use dioxus::prelude::*;
use std::sync::Arc;

use crate::models::user::User;

#[derive(Clone, Copy)]
pub struct UserContext {
    pub user: Signal<Option<Arc<User>>>,
    pub checked: Signal<bool>,
}

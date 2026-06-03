use dioxus::prelude::*;
use std::sync::Arc;

use crate::models::user::PublicUser;

#[derive(Clone, Copy)]
pub struct UserContext {
    pub user: Signal<Option<Arc<PublicUser>>>,
    pub checked: Signal<bool>,
}

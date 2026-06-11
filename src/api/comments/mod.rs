#![allow(clippy::unused_unit, deprecated, unused_imports, clippy::too_many_arguments)]

mod types;
mod helpers;
mod markdown;
mod create;
mod read;
mod update;
mod list;
mod check;

pub use types::*;
pub use create::create_comment;
pub use read::{get_comments, get_comment_count};
pub use update::{approve_comment, spam_comment, trash_comment, batch_update_comment_status};
pub use list::{get_pending_comments, get_pending_count, get_all_comments};
pub use check::check_pending_status;

#[cfg(feature = "server")]
pub use markdown::render_comment_markdown;

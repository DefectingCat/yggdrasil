#![allow(
    clippy::unused_unit,
    deprecated,
    unused_imports,
    clippy::too_many_arguments
)]

mod check;
mod create;
mod helpers;
mod list;
mod markdown;
mod read;
mod types;
mod update;

pub use check::check_pending_status;
pub use create::create_comment;
pub use list::{get_all_comments, get_pending_comments, get_pending_count};
pub use read::{get_comment_count, get_comments};
pub use types::*;
pub use update::{approve_comment, batch_update_comment_status, spam_comment, trash_comment};

#[cfg(feature = "server")]
pub use markdown::render_comment_markdown;

#![allow(clippy::unused_unit, deprecated, unused_imports, clippy::too_many_arguments)]

mod types;
mod helpers;
mod create;
mod update;
mod delete;
mod read;
mod list;
mod search;
mod tags;
mod stats;
mod rebuild;

pub use types::*;
pub use create::create_post;
pub use update::update_post;
pub use delete::delete_post;
pub use read::{get_post_by_id, get_post_by_slug};
pub use list::{list_published_posts, list_posts, get_posts_by_tag};
pub use search::search_posts;
pub use tags::list_tags;
pub use stats::get_post_stats;
pub use rebuild::rebuild_content_html;

#[cfg(feature = "server")]
pub use crate::api::markdown::render_markdown_enhanced;
#[cfg(feature = "server")]
pub use crate::api::slug::{ensure_unique_slug, is_valid_slug, slugify};

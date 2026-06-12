#![allow(clippy::unused_unit, deprecated)]

mod create;
mod delete;
mod helpers;
mod list;
mod read;
mod rebuild;
mod search;
mod stats;
mod tags;
mod types;
mod update;

#[allow(unused_imports)]
pub use create::create_post;
pub use delete::delete_post;
#[allow(unused_imports)]
pub use list::list_posts;
pub use list::{get_posts_by_tag, list_published_posts};
#[allow(unused_imports)]
pub use read::{get_post_by_id, get_post_by_slug};
pub use rebuild::rebuild_content_html;
pub use search::search_posts;
#[allow(unused_imports)]
pub use stats::get_post_stats;
pub use tags::list_tags;
pub use types::*;
#[allow(unused_imports)]
pub use update::update_post;

#[cfg(feature = "server")]
#[allow(unused_imports)]
pub use crate::api::markdown::render_markdown_enhanced;
#[cfg(feature = "server")]
#[allow(unused_imports)]
pub use crate::api::slug::{ensure_unique_slug, is_valid_slug, slugify};

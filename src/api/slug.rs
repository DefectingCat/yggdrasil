#![allow(clippy::unused_unit, deprecated, unused_imports)]

use dioxus::prelude::*;

#[cfg(feature = "server")]
pub fn slugify(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();

    let parts: Vec<&str> = slug.split('-').filter(|s| !s.is_empty()).collect();
    let slug = parts.join("-");

    if slug.is_empty() {
        return format!("{}", chrono::Utc::now().timestamp());
    }

    slug.chars().take(100).collect()
}

#[cfg(feature = "server")]
pub fn is_valid_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > 200 {
        return false;
    }
    slug.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

#[cfg(feature = "server")]
pub async fn ensure_unique_slug(
    client: &tokio_postgres::Client,
    base: &str,
    exclude_id: Option<i32>,
) -> Result<String, ServerFnError> {
    use crate::api::utils::query_error;

    let mut candidate = base.to_string();
    let mut suffix = 2;

    loop {
        let exists = if let Some(exclude) = exclude_id {
            client
                .query_opt(
                    "SELECT 1 FROM posts WHERE slug = $1 AND deleted_at IS NULL AND id != $2",
                    &[&candidate, &exclude],
                )
                .await
                .map_err(query_error)?
                .is_some()
        } else {
            client
                .query_opt(
                    "SELECT 1 FROM posts WHERE slug = $1 AND deleted_at IS NULL",
                    &[&candidate],
                )
                .await
                .map_err(query_error)?
                .is_some()
        };

        if !exists {
            return Ok(candidate);
        }

        candidate = format!("{}-{}", base, suffix);
        suffix += 1;

        if candidate.len() > 200 {
            return Err(ServerFnError::new("无法生成唯一 slug"));
        }
    }
}

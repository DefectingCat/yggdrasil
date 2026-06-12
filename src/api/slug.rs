#![allow(clippy::unused_unit, deprecated)]

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
    use crate::api::error::AppError;

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
                .map_err(AppError::query)?
                .is_some()
        } else {
            client
                .query_opt(
                    "SELECT 1 FROM posts WHERE slug = $1 AND deleted_at IS NULL",
                    &[&candidate],
                )
                .await
                .map_err(AppError::query)?
                .is_some()
        };

        if !exists {
            return Ok(candidate);
        }

        candidate = format!("{}-{}", base, suffix);
        suffix += 1;

        if candidate.len() > 200 {
            return Err(AppError::Internal("无法生成唯一 slug").into());
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn slugify_ascii_title() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn slugify_special_characters() {
        assert_eq!(slugify("Hello, World! (2024)"), "hello-world-2024");
    }

    #[test]
    fn slugify_chinese_characters() {
        let slug = slugify("你好世界 hello");
        assert!(slug.contains("hello"));
    }

    #[test]
    fn slugify_collapses_dashes() {
        assert_eq!(slugify("a---b"), "a-b");
    }

    #[test]
    fn slugify_empty_returns_timestamp() {
        let slug = slugify("");
        let _: i64 = slug.parse().expect("should be a valid timestamp");
    }

    #[test]
    fn slugify_truncates_at_100_chars() {
        let long_title = "a".repeat(200);
        assert!(slugify(&long_title).len() <= 100);
    }

    #[test]
    fn slugify_preserves_underscores() {
        assert_eq!(slugify("hello_world"), "hello_world");
    }

    #[test]
    fn is_valid_slug_normal() {
        assert!(is_valid_slug("hello-world_123"));
    }

    #[test]
    fn is_valid_slug_rejects_empty() {
        assert!(!is_valid_slug(""));
    }

    #[test]
    fn is_valid_slug_rejects_too_long() {
        let long_slug = "a".repeat(201);
        assert!(!is_valid_slug(&long_slug));
    }

    #[test]
    fn is_valid_slug_accepts_max_length() {
        let slug = "a".repeat(200);
        assert!(is_valid_slug(&slug));
    }

    #[test]
    fn is_valid_slug_rejects_special_chars() {
        assert!(!is_valid_slug("hello world"));
        assert!(!is_valid_slug("hello.world"));
        assert!(!is_valid_slug("hello!world"));
    }

    #[test]
    fn is_valid_slug_accepts_chinese() {
        assert!(is_valid_slug("你好-world"));
    }

    #[test]
    fn slugify_all_special_characters_returns_timestamp() {
        let slug = slugify("!@#$%^&*()+=[]{}|\\;:'\",.<>/?`~");
        let _: i64 = slug.parse().expect("should be a valid timestamp");
    }

    #[test]
    fn slugify_only_whitespace_returns_timestamp() {
        let slug = slugify("   \t\n  ");
        let _: i64 = slug.parse().expect("should be a valid timestamp");
    }

    #[test]
    fn slugify_leading_and_trailing_dashes() {
        assert_eq!(slugify("-hello-world-"), "hello-world");
        assert_eq!(slugify("---hello---world---"), "hello-world");
    }

    #[test]
    fn is_valid_slug_mixed_chinese_and_digits() {
        assert!(is_valid_slug("你好123"));
        assert!(is_valid_slug("123你好456"));
    }

    #[test]
    fn is_valid_slug_exact_200_char_boundary() {
        let slug = "a".repeat(200);
        assert!(is_valid_slug(&slug));
        let slug = "a".repeat(201);
        assert!(!is_valid_slug(&slug));
    }
}

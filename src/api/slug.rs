//! 文章 slug 生成与唯一性校验。
//!
//! 将标题转换为小写、仅含字母数字与连字符/下划线的 URL 友好形式，
//! 并检测数据库中是否已存在，必要时追加数字后缀。
//! 仅在 `feature = "server"` 时访问数据库。

#![allow(clippy::unused_unit, deprecated)]

#[cfg(feature = "server")]
use dioxus::prelude::*;

#[cfg(feature = "server")]
use pinyin::ToPinyin;

#[cfg(feature = "server")]
/// 将标题转换为 URL 友好的 slug。
///
/// 汉字转为无声调拼音（每字独立成词，用 `-` 分隔，如 `你好` → `ni-hao`）；
/// ASCII 字母数字与 `-`/`_` 保留；其余字符替换为 `-`，连续 `-` 合并。
/// 结果截断至 100 字符；若全部被过滤则回退为当前时间戳。
///
/// 多音字取默认读音（不处理歧义），博客 slug 场景足够。
pub fn slugify(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    for c in title.to_lowercase().chars() {
        // 汉字优先转拼音：to_pinyin() 对非汉字（含 ascii）返回 None。
        if let Some(py) = c.to_pinyin() {
            out.push_str(py.plain());
            // 每个汉字成词，用 `-` 与后续内容分隔。
            out.push('-');
        } else if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else {
            out.push('-');
        }
    }

    // 合并连续的连字符，并去除首尾空段。
    let parts: Vec<&str> = out.split('-').filter(|s| !s.is_empty()).collect();
    let slug = parts.join("-");

    if slug.is_empty() {
        return format!("{}", chrono::Utc::now().timestamp());
    }

    slug.chars().take(100).collect()
}

#[cfg(feature = "server")]
/// 校验 slug 是否为空且仅含合法字符、长度不超过 200。
pub fn is_valid_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > 200 {
        return false;
    }
    slug.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

#[cfg(feature = "server")]
/// 确保生成的 slug 在数据库中唯一。
///
/// 若 `exclude_id` 不为空，则排除该文章自身；
/// 当冲突时依次尝试 `base-2`、`base-3` …… 直到生成唯一值。
///
/// 该函数应在事务内调用，确保与后续 INSERT/UPDATE 的 slug 唯一性检查
/// 在同一个事务中完成，避免并发竞态。
pub async fn ensure_unique_slug(
    tx: &deadpool_postgres::Transaction<'_>,
    base: &str,
    exclude_id: Option<i32>,
) -> Result<String, ServerFnError> {
    use crate::api::error::AppError;

    let mut candidate = base.to_string();
    let mut suffix = 2;

    loop {
        // 查询当前候选 slug 是否已存在（排除指定文章 ID）。
        let exists = if let Some(exclude) = exclude_id {
            tx.query_opt(
                "SELECT 1 FROM posts WHERE slug = $1 AND deleted_at IS NULL AND id != $2",
                &[&candidate, &exclude],
            )
            .await
            .map_err(AppError::query)?
            .is_some()
        } else {
            tx.query_opt(
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

        // 防止无限循环：slug 总长度超过 200 时直接报错。
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
        // 汉字逐字转拼音，用 `-` 分隔；混入的 ascii 自然衔接到末尾。
        assert_eq!(slugify("你好世界"), "ni-hao-shi-jie");
        assert_eq!(slugify("你好世界 hello"), "ni-hao-shi-jie-hello");
    }

    #[test]
    fn slugify_mixed_chinese_ascii() {
        // Rust 入门指南 → rust + ru-men-zhi-nan
        assert_eq!(slugify("Rust 入门指南"), "rust-ru-men-zhi-nan");
    }

    #[test]
    fn slugify_chinese_with_punctuation() {
        // 标点替换为 `-`，随后与拼音分隔符合并。
        assert_eq!(slugify("你好，世界！"), "ni-hao-shi-jie");
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

#![allow(clippy::unused_unit, deprecated, unused_imports)]

#[cfg(feature = "server")]
use crate::models::comment::{AdminComment, CommentStatus, PublicComment};

#[cfg(feature = "server")]
pub fn md5_hash(input: &str) -> String {
    use md5::Digest;
    let hash = md5::Md5::digest(input.as_bytes());
    hex::encode(hash)
}

#[cfg(feature = "server")]
pub fn gravatar_url(email: &str) -> String {
    let hash = md5_hash(&email.trim().to_lowercase());
    format!("https://cravatar.cn/avatar/{}?d=mp&s=80", hash)
}

#[cfg(feature = "server")]
pub fn row_to_public_comment(row: &tokio_postgres::Row) -> PublicComment {
    let email: String = row.get("author_email");
    let created_at_dt: chrono::DateTime<chrono::Utc> = row.get("created_at");
    let created_at_iso = created_at_dt.to_rfc3339();
    let created_at_relative = format_relative_time(created_at_dt);

    PublicComment {
        id: row.get("id"),
        parent_id: row.get("parent_id"),
        depth: row.get("depth"),
        author_name: row.get("author_name"),
        author_url: row.get("author_url"),
        avatar_url: gravatar_url(&email),
        content_html: row.get("content_html"),
        created_at: created_at_relative,
        created_at_iso,
    }
}

#[cfg(feature = "server")]
pub fn row_to_admin_comment(row: &tokio_postgres::Row) -> AdminComment {
    let status_str: String = row.get("status");

    AdminComment {
        id: row.get("id"),
        post_id: row.get("post_id"),
        post_title: row.get("post_title"),
        post_slug: row.get("post_slug"),
        parent_id: row.get("parent_id"),
        depth: row.get("depth"),
        author_name: row.get("author_name"),
        author_email: row.get("author_email"),
        author_url: row.get("author_url"),
        content_md: row.get("content_md"),
        status: CommentStatus::from_str(&status_str),
        created_at: row.get("created_at"),
    }
}

pub fn format_relative_time(dt: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(dt);

    if diff.num_seconds() < 60 {
        "刚刚".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{} 分钟前", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{} 小时前", diff.num_hours())
    } else if diff.num_days() < 30 {
        format!("{} 天前", diff.num_days())
    } else {
        dt.format("%Y-%m-%d").to_string()
    }
}

#[allow(dead_code)]
pub fn validate_comment_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("请输入昵称".to_string());
    }
    if trimmed.len() > 50 {
        return Err("昵称长度不能超过 50 个字符".to_string());
    }
    Ok(())
}

#[allow(dead_code)]
pub fn validate_comment_email(email: &str) -> Result<(), String> {
    let re =
        regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    if !re.is_match(email.trim()) {
        return Err("邮箱格式不正确".to_string());
    }
    Ok(())
}

#[allow(dead_code)]
pub fn validate_comment_url(url: &str) -> Result<(), String> {
    if url.trim().is_empty() {
        return Ok(());
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("网址必须以 http:// 或 https:// 开头".to_string());
    }
    if url.len() > 200 {
        return Err("网址长度不能超过 200 个字符".to_string());
    }
    Ok(())
}

#[allow(dead_code)]
pub fn validate_comment_content(content: &str) -> Result<(), String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("请输入评论内容".to_string());
    }
    if trimmed.len() > 10000 {
        return Err("评论内容不能超过 10000 个字符".to_string());
    }
    Ok(())
}

pub fn compute_content_hash(
    post_id: i32,
    parent_id: Option<i64>,
    name: &str,
    content: &str,
) -> String {
    use sha2::Digest;
    let input = format!(
        "{}:{}:{}:{}",
        post_id,
        parent_id.map(|id| id.to_string()).unwrap_or_default(),
        name.trim(),
        content.trim()
    );
    let hash = sha2::Sha256::digest(input.as_bytes());
    hex::encode(hash)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn md5_hash_known_value() {
        assert_eq!(md5_hash("hello"), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn md5_hash_empty() {
        assert_eq!(md5_hash(""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn gravatar_url_format() {
        let url = gravatar_url("test@example.com");
        assert!(url.starts_with("https://cravatar.cn/avatar/"));
        assert!(url.contains("?d=mp&s=80"));
    }

    #[test]
    fn gravatar_url_normalizes_email() {
        let url1 = gravatar_url("Test@Example.com");
        let url2 = gravatar_url("test@example.com");
        assert_eq!(url1, url2);
    }

    #[test]
    fn gravatar_url_trims_whitespace() {
        let url1 = gravatar_url(" test@example.com ");
        let url2 = gravatar_url("test@example.com");
        assert_eq!(url1, url2);
    }

    #[test]
    fn format_relative_time_just_now() {
        let now = chrono::Utc::now();
        assert_eq!(format_relative_time(now), "刚刚");
    }

    #[test]
    fn format_relative_time_minutes() {
        let dt = chrono::Utc::now() - chrono::Duration::minutes(5);
        assert_eq!(format_relative_time(dt), "5 分钟前");
    }

    #[test]
    fn format_relative_time_hours() {
        let dt = chrono::Utc::now() - chrono::Duration::hours(3);
        assert_eq!(format_relative_time(dt), "3 小时前");
    }

    #[test]
    fn format_relative_time_days() {
        let dt = chrono::Utc::now() - chrono::Duration::days(7);
        assert_eq!(format_relative_time(dt), "7 天前");
    }

    #[test]
    fn format_relative_time_old_date() {
        let dt = chrono::Utc::now() - chrono::Duration::days(60);
        let result = format_relative_time(dt);
        assert!(result.contains('-'));
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn validate_comment_name_valid() {
        assert!(validate_comment_name("Alice").is_ok());
        assert!(validate_comment_name("张三").is_ok());
    }

    #[test]
    fn validate_comment_name_empty() {
        assert!(validate_comment_name("").is_err());
        assert!(validate_comment_name("   ").is_err());
    }

    #[test]
    fn validate_comment_name_too_long() {
        assert!(validate_comment_name(&"a".repeat(51)).is_err());
    }

    #[test]
    fn validate_comment_name_max_length() {
        assert!(validate_comment_name(&"a".repeat(50)).is_ok());
    }

    #[test]
    fn validate_comment_email_valid() {
        assert!(validate_comment_email("user@example.com").is_ok());
        assert!(validate_comment_email("a.b+c@domain.co").is_ok());
    }

    #[test]
    fn validate_comment_email_invalid() {
        assert!(validate_comment_email("notanemail").is_err());
        assert!(validate_comment_email("@domain.com").is_err());
        assert!(validate_comment_email("user@").is_err());
    }

    #[test]
    fn validate_comment_url_valid() {
        assert!(validate_comment_url("http://example.com").is_ok());
        assert!(validate_comment_url("https://example.com/path").is_ok());
    }

    #[test]
    fn validate_comment_url_empty_is_ok() {
        assert!(validate_comment_url("").is_ok());
        assert!(validate_comment_url("   ").is_ok());
    }

    #[test]
    fn validate_comment_url_invalid_scheme() {
        assert!(validate_comment_url("ftp://example.com").is_err());
        assert!(validate_comment_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn validate_comment_url_too_long() {
        let long_url = format!("https://example.com/{}", "a".repeat(200));
        assert!(validate_comment_url(&long_url).is_err());
    }

    #[test]
    fn validate_comment_content_valid() {
        assert!(validate_comment_content("Hello world").is_ok());
    }

    #[test]
    fn validate_comment_content_empty() {
        assert!(validate_comment_content("").is_err());
        assert!(validate_comment_content("   ").is_err());
    }

    #[test]
    fn validate_comment_content_too_long() {
        assert!(validate_comment_content(&"a".repeat(10001)).is_err());
    }

    #[test]
    fn validate_comment_content_max_length() {
        assert!(validate_comment_content(&"a".repeat(10000)).is_ok());
    }

    #[test]
    fn compute_content_hash_deterministic() {
        let h1 = compute_content_hash(1, None, "Alice", "Hello");
        let h2 = compute_content_hash(1, None, "Alice", "Hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn compute_content_hash_different_inputs() {
        let h1 = compute_content_hash(1, None, "Alice", "Hello");
        let h2 = compute_content_hash(2, None, "Alice", "Hello");
        assert_ne!(h1, h2);
    }

    #[test]
    fn compute_content_hash_trims_whitespace() {
        let h1 = compute_content_hash(1, None, "Alice", "Hello");
        let h2 = compute_content_hash(1, None, " Alice ", " Hello ");
        assert_eq!(h1, h2);
    }

    #[test]
    fn compute_content_hash_64_hex_chars() {
        let h = compute_content_hash(1, None, "Alice", "Hello");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

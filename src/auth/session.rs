use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[allow(dead_code)]
pub fn generate_token() -> String {
    Uuid::new_v4().to_string()
}

#[allow(dead_code)]
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[allow(dead_code)]
pub fn default_expiry() -> DateTime<Utc> {
    Utc::now() + Duration::days(30)
}

#[cfg(feature = "server")]
pub fn parse_session_token(cookie_header: &str) -> Option<&str> {
    cookie_header.split(';').map(|s| s.trim()).find_map(|pair| {
        let mut parts = pair.splitn(2, '=');
        let name = parts.next()?.trim();
        let value = parts.next()?.trim();
        if name == "session" {
            Some(value)
        } else {
            None
        }
    })
}

#[cfg(feature = "server")]
pub fn get_session_from_ctx() -> Option<String> {
    use dioxus::fullstack::FullstackContext;

    FullstackContext::current().and_then(|ctx| {
        let parts = ctx.parts_mut();
        parts
            .headers
            .get("cookie")
            .and_then(|h| h.to_str().ok())
            .and_then(parse_session_token)
            .map(|s| s.to_string())
    })
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn parse_session_found() {
        let header = "session=abc123; path=/";
        assert_eq!(parse_session_token(header), Some("abc123"));
    }

    #[test]
    fn parse_session_single_cookie() {
        assert_eq!(parse_session_token("session=token456"), Some("token456"));
    }

    #[test]
    fn parse_session_not_found() {
        assert_eq!(parse_session_token("other=value"), None);
    }

    #[test]
    fn parse_session_empty_string() {
        assert_eq!(parse_session_token(""), None);
    }

    #[test]
    fn parse_session_multiple_cookies() {
        let header = "theme=dark; session=my-secret; lang=en";
        assert_eq!(parse_session_token(header), Some("my-secret"));
    }

    #[test]
    fn parse_session_empty_value() {
        assert_eq!(parse_session_token("session="), Some(""));
    }

    #[test]
    fn parse_session_trailing_semicolon() {
        assert_eq!(parse_session_token("session=abc;"), Some("abc"));
    }

    #[test]
    fn generate_token_is_uuid() {
        let token = generate_token();
        assert!(uuid::Uuid::parse_str(&token).is_ok());
    }

    #[test]
    fn default_expiry_is_future() {
        let expiry = default_expiry();
        assert!(expiry > chrono::Utc::now());
    }

    #[test]
    fn default_expiry_is_about_30_days() {
        let expiry = default_expiry();
        let diff = expiry - chrono::Utc::now();
        assert!(diff.num_days() >= 29 && diff.num_days() <= 31);
    }

    #[test]
    fn hash_token_is_deterministic() {
        let token = "test-token-123";
        assert_eq!(hash_token(token), hash_token(token));
    }

    #[test]
    fn hash_token_is_64_chars() {
        let hash = hash_token("any-token");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn hash_token_differs_from_input() {
        let token = "my-secret-token";
        assert_ne!(hash_token(token), token);
    }

    #[test]
    fn hash_token_known_value() {
        let hash = hash_token("hello");
        let expected = sha2::Sha256::digest(b"hello");
        assert_eq!(hash, hex::encode(expected));
    }
}

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

#[allow(dead_code)]
pub fn generate_token() -> String {
    Uuid::new_v4().to_string()
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

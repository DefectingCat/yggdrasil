#[cfg(feature = "server")]
use axum::http::StatusCode;
#[cfg(feature = "server")]
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
#[cfg(feature = "server")]
use std::num::NonZeroU32;
#[cfg(feature = "server")]
use std::sync::LazyLock;

#[cfg(feature = "server")]
fn env_or(key: &str, default: u32) -> NonZeroU32 {
    let val = std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default);
    NonZeroU32::new(val.max(1)).unwrap()
}

#[cfg(feature = "server")]
static STRICT_LIMITER: LazyLock<DefaultKeyedRateLimiter<String>> = LazyLock::new(|| {
    RateLimiter::keyed(
        Quota::per_second(env_or("RATE_LIMIT_STRICT_PER_SEC", 1))
            .allow_burst(env_or("RATE_LIMIT_STRICT_BURST", 5)),
    )
});

#[cfg(feature = "server")]
static UPLOAD_LIMITER: LazyLock<DefaultKeyedRateLimiter<String>> = LazyLock::new(|| {
    RateLimiter::keyed(
        Quota::per_second(env_or("RATE_LIMIT_UPLOAD_PER_SEC", 2))
            .allow_burst(env_or("RATE_LIMIT_UPLOAD_BURST", 15)),
    )
});

#[cfg(feature = "server")]
static IMAGE_LIMITER: LazyLock<DefaultKeyedRateLimiter<String>> = LazyLock::new(|| {
    RateLimiter::keyed(
        Quota::per_second(env_or("RATE_LIMIT_IMAGE_PER_SEC", 10))
            .allow_burst(env_or("RATE_LIMIT_IMAGE_BURST", 50)),
    )
});

#[cfg(feature = "server")]
static COMMENT_LIMITER: LazyLock<DefaultKeyedRateLimiter<String>> = LazyLock::new(|| {
    RateLimiter::keyed(
        Quota::per_second(env_or("RATE_LIMIT_COMMENT_PER_SEC", 1))
            .allow_burst(env_or("RATE_LIMIT_COMMENT_BURST", 5)),
    )
});

#[cfg(feature = "server")]
pub fn check_comment_limit(ip: &str) -> Result<(), String> {
    COMMENT_LIMITER
        .check_key(&ip.to_string())
        .map(|_| ())
        .map_err(|_| "评论过于频繁，请稍后再试".to_string())
}

#[cfg(feature = "server")]
pub fn check_image_limit(ip: &str) -> Result<(), StatusCode> {
    IMAGE_LIMITER
        .check_key(&ip.to_string())
        .map(|_| ())
        .map_err(|_| StatusCode::TOO_MANY_REQUESTS)
}

#[cfg(feature = "server")]
fn trusted_proxy_count() -> usize {
    std::env::var("TRUSTED_PROXY_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

#[cfg(feature = "server")]
fn ip_from_x_forwarded_for(value: &str, trusted_proxy_count: usize) -> Option<String> {
    let parts: Vec<&str> = value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() || trusted_proxy_count == 0 {
        return None;
    }
    if parts.len() <= trusted_proxy_count {
        return None;
    }
    let idx = parts.len() - 1 - trusted_proxy_count;
    parts.get(idx).map(|s| s.to_string())
}

#[cfg(feature = "server")]
pub fn get_client_ip_with_trusted(headers: &http::HeaderMap, trusted_proxy_count: usize) -> String {
    if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(ip) = ip_from_x_forwarded_for(value, trusted_proxy_count) {
            return ip;
        }
    }

    if trusted_proxy_count > 0 {
        if let Some(ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
            return ip.trim().to_string();
        }
    }

    "unknown".to_string()
}

#[cfg(feature = "server")]
pub fn get_client_ip(headers: &http::HeaderMap) -> String {
    get_client_ip_with_trusted(headers, trusted_proxy_count())
}

#[cfg(feature = "server")]
pub fn check_strict_limit(ip: &str) -> Result<(), String> {
    STRICT_LIMITER
        .check_key(&ip.to_string())
        .map(|_| ())
        .map_err(|_| "请求过于频繁，请稍后再试".to_string())
}

#[cfg(feature = "server")]
pub fn check_upload_limit(ip: &str) -> Result<(), String> {
    UPLOAD_LIMITER
        .check_key(&ip.to_string())
        .map(|_| ())
        .map_err(|_| "上传过于频繁，请稍后再试".to_string())
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use http::HeaderMap;

    #[test]
    fn get_client_ip_from_x_forwarded_for_with_one_trusted_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(get_client_ip_with_trusted(&headers, 1), "1.2.3.4");
    }

    #[test]
    fn get_client_ip_from_x_forwarded_for_with_two_trusted_proxies() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "1.2.3.4, 5.6.7.8, 9.10.11.12".parse().unwrap(),
        );
        assert_eq!(get_client_ip_with_trusted(&headers, 2), "1.2.3.4");
    }

    #[test]
    fn get_client_ip_ignores_x_forwarded_for_when_no_trusted_proxies() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(get_client_ip_with_trusted(&headers, 0), "unknown");
    }

    #[test]
    fn get_client_ip_from_x_real_ip_when_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "9.8.7.6".parse().unwrap());
        assert_eq!(get_client_ip_with_trusted(&headers, 1), "9.8.7.6");
    }

    #[test]
    fn get_client_ip_x_real_ip_ignored_when_not_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "9.8.7.6".parse().unwrap());
        assert_eq!(get_client_ip_with_trusted(&headers, 0), "unknown");
    }

    #[test]
    fn get_client_ip_x_forwarded_for_takes_priority_over_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.1.1.1, 2.2.2.2".parse().unwrap());
        headers.insert("x-real-ip", "3.3.3.3".parse().unwrap());
        assert_eq!(get_client_ip_with_trusted(&headers, 1), "1.1.1.1");
    }

    #[test]
    fn get_client_ip_no_headers_returns_unknown() {
        let headers = HeaderMap::new();
        assert_eq!(get_client_ip_with_trusted(&headers, 1), "unknown");
    }

    #[test]
    fn get_client_ip_ignores_short_x_forwarded_for_list() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4".parse().unwrap());
        assert_eq!(get_client_ip_with_trusted(&headers, 2), "unknown");
    }

    #[test]
    fn get_client_ip_ignores_x_forwarded_for_equal_to_proxy_count() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(get_client_ip_with_trusted(&headers, 2), "unknown");
    }

    #[test]
    fn get_client_ip_ignores_empty_x_forwarded_for_entries() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            " , 1.2.3.4 , 5.6.7.8 , ".parse().unwrap(),
        );
        assert_eq!(get_client_ip_with_trusted(&headers, 1), "1.2.3.4");
    }
}

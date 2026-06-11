#![allow(clippy::unused_unit)]

#[cfg(feature = "server")]
use std::sync::LazyLock;
#[cfg(feature = "server")]
use std::num::NonZeroU32;
#[cfg(feature = "server")]
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
#[cfg(feature = "server")]
use axum::http::StatusCode;

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
pub fn get_client_ip(headers: &http::HeaderMap) -> String {
    if let Some(ip) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
    {
        return ip.trim().to_string();
    }
    if let Some(ip) = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
    {
        return ip.trim().to_string();
    }
    "unknown".to_string()
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
    fn get_client_ip_from_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(get_client_ip(&headers), "1.2.3.4");
    }

    #[test]
    fn get_client_ip_from_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "9.8.7.6".parse().unwrap());
        assert_eq!(get_client_ip(&headers), "9.8.7.6");
    }

    #[test]
    fn get_client_ip_x_forwarded_for_takes_priority() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.1.1.1".parse().unwrap());
        headers.insert("x-real-ip", "2.2.2.2".parse().unwrap());
        assert_eq!(get_client_ip(&headers), "1.1.1.1");
    }

    #[test]
    fn get_client_ip_no_headers_returns_unknown() {
        let headers = HeaderMap::new();
        assert_eq!(get_client_ip(&headers), "unknown");
    }
}

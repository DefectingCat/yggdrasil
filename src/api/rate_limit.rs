#![allow(clippy::unused_unit)]

#[cfg(feature = "server")]
use std::sync::Arc;
#[cfg(feature = "server")]
use std::sync::LazyLock;
#[cfg(feature = "server")]
use std::num::NonZeroU32;
#[cfg(feature = "server")]
use tower_governor::governor::GovernorConfigBuilder;
#[cfg(feature = "server")]
use tower_governor::GovernorLayer;
#[cfg(feature = "server")]
use tower_governor::key_extractor::SmartIpKeyExtractor;
#[cfg(feature = "server")]
use governor::middleware::NoOpMiddleware;
#[cfg(feature = "server")]
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};

/// 通用限流配置：每秒 1 请求，突发 30
#[cfg(feature = "server")]
pub fn general_limit() -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware> {
    let config = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(30)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .unwrap();
    GovernorLayer {
        config: Arc::new(config),
    }
}

// 严格限流：每秒 1，突发 5（用于登录、注册等敏感操作）
#[cfg(feature = "server")]
static STRICT_LIMITER: LazyLock<DefaultKeyedRateLimiter<String>> = LazyLock::new(|| {
    RateLimiter::keyed(
        Quota::per_second(NonZeroU32::new(1).unwrap())
            .allow_burst(NonZeroU32::new(5).unwrap())
    )
});

// 上传限流：每秒 1，突发 10
#[cfg(feature = "server")]
static UPLOAD_LIMITER: LazyLock<DefaultKeyedRateLimiter<String>> = LazyLock::new(|| {
    RateLimiter::keyed(
        Quota::per_second(NonZeroU32::new(1).unwrap())
            .allow_burst(NonZeroU32::new(10).unwrap())
    )
});

/// 从请求 headers 中提取客户端 IP
#[cfg(feature = "server")]
pub fn get_client_ip(headers: &http::HeaderMap) -> String {
    // 1. X-Forwarded-For
    if let Some(ip) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
    {
        return ip.trim().to_string();
    }
    // 2. X-Real-Ip
    if let Some(ip) = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
    {
        return ip.trim().to_string();
    }
    "unknown".to_string()
}

/// 检查严格限流（用于登录、注册等敏感操作）
#[cfg(feature = "server")]
pub fn check_strict_limit(ip: &str) -> Result<(), String> {
    STRICT_LIMITER
        .check_key(&ip.to_string())
        .map(|_| ())
        .map_err(|_| "请求过于频繁，请稍后再试".to_string())
}

/// 检查上传限流
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

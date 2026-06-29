//! 基于 governor 的多级限流。
//!
//! 提供 strict、upload、image、comment 四个限流器，
//! 支持从 `X-Forwarded-For` / `X-Real-IP` 中提取客户端 IP，
//! 并可通过 `TRUSTED_PROXY_COUNT` 配置信任代理层数。
//!
//! 当未配置可信代理时，Axum handler 可回退到 TCP 连接的对端地址；
//! Dioxus server function 无法获取对端地址，会退回到 `"unknown"` key，
//! 此时所有请求共享一个限流桶。生产环境应在反向代理后部署并正确配置
//! `TRUSTED_PROXY_COUNT`。
//!
//! 仅在 `feature = "server"` 时生效。

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
/// 当无法识别真实客户端 IP（"unknown"）时使用的宽松限流桶。
///
/// TRUSTED_PROXY_COUNT=0（默认）时，Dioxus server function 拿不到 TCP 对端地址，
/// get_client_ip 会返回 "unknown"，导致所有匿名请求共享同一个严格桶
/// （1 req/s, burst 5），正常用户的高频请求被误杀。此桶阈值更高，
/// 通过 env RATE_LIMIT_UNKNOWN_PER_SEC / RATE_LIMIT_UNKNOWN_BURST 可调。
static UNKNOWN_BUCKET_LIMITER: LazyLock<DefaultKeyedRateLimiter<String>> = LazyLock::new(|| {
    RateLimiter::keyed(
        Quota::per_second(env_or("RATE_LIMIT_UNKNOWN_PER_SEC", 30))
            .allow_burst(env_or("RATE_LIMIT_UNKNOWN_BURST", 100)),
    )
});

#[cfg(feature = "server")]
/// 检查评论请求是否超出限流阈值。
pub fn check_comment_limit(ip: &str) -> Result<(), String> {
    COMMENT_LIMITER
        .check_key(&ip.to_string())
        .map(|_| ())
        .map_err(|_| "评论过于频繁，请稍后再试".to_string())
}

#[cfg(feature = "server")]
/// 检查图片访问请求是否超出限流阈值，返回 HTTP 状态码。
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
fn is_valid_ip(ip: &str) -> bool {
    ip.parse::<std::net::IpAddr>().is_ok()
}

#[cfg(feature = "server")]
fn ip_from_x_forwarded_for(value: &str, trusted_proxy_count: usize) -> Option<String> {
    // X-Forwarded-For 格式：client, proxy1, proxy2, ..., proxyN
    // 越靠右的地址离服务端越近。
    let parts: Vec<&str> = value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if trusted_proxy_count == 0 || parts.len() <= trusted_proxy_count {
        return None;
    }

    // 真实客户端 IP 位于右侧第 trusted_proxy_count + 1 个。
    let idx = parts.len() - 1 - trusted_proxy_count;
    let ip = parts[idx].to_string();
    if is_valid_ip(&ip) {
        Some(ip)
    } else {
        None
    }
}

#[cfg(feature = "server")]
fn ip_from_x_real_ip(value: &str) -> Option<String> {
    let ip = value.trim().to_string();
    if is_valid_ip(&ip) {
        Some(ip)
    } else {
        None
    }
}

#[cfg(feature = "server")]
fn get_client_ip_internal(
    headers: &http::HeaderMap,
    trusted: usize,
    peer: Option<std::net::SocketAddr>,
) -> String {
    if trusted > 0 {
        if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            if let Some(ip) = ip_from_x_forwarded_for(value, trusted) {
                return ip;
            }
        }

        if let Some(ip) = headers
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(ip_from_x_real_ip)
        {
            return ip;
        }
    }

    if let Some(addr) = peer {
        return addr.ip().to_string();
    }

    // Server function 等非 Axum 上下文无法获取对端地址，退回到 unknown。
    // 此时所有请求共享一个限流桶，生产环境应在反向代理后部署。
    tracing::warn!(
        "无法获取客户端真实 IP（未配置 TRUSTED_PROXY_COUNT 且无法读取 TCP 对端地址），\
         限流将按 'unknown' 键聚合"
    );
    "unknown".to_string()
}

#[cfg(feature = "server")]
/// 根据信任代理层数从请求头中提取客户端 IP，并校验 IP 合法性。
///
/// 当未配置可信代理时，不会信任任何 `X-Forwarded-For` / `X-Real-IP` 头，
/// 而是直接返回 `peer` 中的 TCP 对端地址（如果提供）。
pub fn get_client_ip_with_peer(
    headers: &http::HeaderMap,
    peer: Option<std::net::SocketAddr>,
) -> String {
    get_client_ip_internal(headers, trusted_proxy_count(), peer)
}

#[cfg(feature = "server")]
/// 使用环境变量配置的代理层数提取客户端 IP。
///
/// 适用于 Dioxus server function 等无法获取 `ConnectInfo` 的场景。
/// 生产环境建议配合反向代理与 `TRUSTED_PROXY_COUNT` 使用。
pub fn get_client_ip(headers: &http::HeaderMap) -> String {
    get_client_ip_internal(headers, trusted_proxy_count(), None)
}

#[cfg(feature = "server")]
/// 检查严格限流（注册、登录等敏感接口）。
///
/// 当 IP 为 "unknown"（无法识别真实客户端，通常是 TRUSTED_PROXY_COUNT=0
/// 且调用方为 Dioxus server function 时）改用宽松桶，避免所有匿名请求共享
/// 严格桶导致正常用户被误杀。生产环境配好 TRUSTED_PROXY_COUNT 后走真实 IP，
/// 始终命中严格桶。
pub fn check_strict_limit(ip: &str) -> Result<(), String> {
    if ip == "unknown" {
        UNKNOWN_BUCKET_LIMITER
            .check_key(&ip.to_string())
            .map(|_| ())
            .map_err(|_| "服务繁忙，请稍后再试".to_string())
    } else {
        STRICT_LIMITER
            .check_key(&ip.to_string())
            .map(|_| ())
            .map_err(|_| "请求过于频繁，请稍后再试".to_string())
    }
}

#[cfg(feature = "server")]
/// 检查上传请求是否超出限流阈值。
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
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn get_client_ip_from_x_forwarded_for_with_one_trusted_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 1, None),
            "1.2.3.4"
        );
    }

    #[test]
    fn get_client_ip_from_x_forwarded_for_with_two_trusted_proxies() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "1.2.3.4, 5.6.7.8, 9.10.11.12".parse().unwrap(),
        );
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 2, None),
            "1.2.3.4"
        );
    }

    #[test]
    fn get_client_ip_ignores_x_forwarded_for_when_no_trusted_proxies() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 0, None),
            "unknown"
        );
    }

    #[test]
    fn get_client_ip_falls_back_to_peer_when_no_trusted_proxies() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        let peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 0, Some(peer)),
            "127.0.0.1"
        );
    }

    #[test]
    fn get_client_ip_from_x_real_ip_when_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "9.8.7.6".parse().unwrap());
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 1, None),
            "9.8.7.6"
        );
    }

    #[test]
    fn get_client_ip_x_real_ip_ignored_when_not_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "9.8.7.6".parse().unwrap());
        let peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 12345);
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 0, Some(peer)),
            "192.168.1.1"
        );
    }

    #[test]
    fn get_client_ip_x_forwarded_for_takes_priority_over_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.1.1.1, 2.2.2.2".parse().unwrap());
        headers.insert("x-real-ip", "3.3.3.3".parse().unwrap());
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 1, None),
            "1.1.1.1"
        );
    }

    #[test]
    fn get_client_ip_no_headers_returns_unknown() {
        let headers = HeaderMap::new();
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 1, None),
            "unknown"
        );
    }

    #[test]
    fn get_client_ip_ignores_short_x_forwarded_for_list() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4".parse().unwrap());
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 2, None),
            "unknown"
        );
    }

    #[test]
    fn get_client_ip_ignores_x_forwarded_for_equal_to_proxy_count() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 2, None),
            "unknown"
        );
    }

    #[test]
    fn get_client_ip_ignores_empty_x_forwarded_for_entries() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            " , 1.2.3.4 , 5.6.7.8 , ".parse().unwrap(),
        );
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 1, None),
            "1.2.3.4"
        );
    }

    #[test]
    fn get_client_ip_rejects_invalid_x_forwarded_for_value() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "not-an-ip, 5.6.7.8".parse().unwrap());
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 1, None),
            "unknown"
        );
    }

    #[test]
    fn get_client_ip_rejects_invalid_x_real_ip_value() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "not-an-ip".parse().unwrap());
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 1, None),
            "unknown"
        );
    }

    #[test]
    fn get_client_ip_prefers_xff_over_peer() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        let peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);
        assert_eq!(
            get_client_ip_with_trusted_and_peer(&headers, 1, Some(peer)),
            "1.2.3.4"
        );
    }

    #[test]
    fn get_client_ip_with_env_trusted_proxy_count_zero() {
        let original = std::env::var("TRUSTED_PROXY_COUNT").ok();
        std::env::set_var("TRUSTED_PROXY_COUNT", "0");

        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(get_client_ip(&headers), "unknown");

        match original {
            Some(value) => std::env::set_var("TRUSTED_PROXY_COUNT", value),
            None => std::env::remove_var("TRUSTED_PROXY_COUNT"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn check_strict_unknown_ip_uses_lenient_bucket() {
        // "unknown" 桶 burst 为 100，少量请求应全部放行，不被严格桶误杀。
        // 用 serial 隔离，因为 UNKNOWN_BUCKET_LIMITER 是全局状态。
        for _ in 0..20 {
            assert!(
                super::check_strict_limit("unknown").is_ok(),
                "unknown bucket should allow small bursts, not hit strict 1 req/s limit"
            );
        }
    }

    #[test]
    #[serial_test::serial]
    fn check_strict_real_ip_uses_strict_bucket() {
        // 真实 IP 命中严格桶（1 req/s, burst 5）。连发超过 burst 应被限流。
        // 用一个唯一的 IP 避免与其他测试状态冲突。
        let unique_ip = "198.51.100.42";
        let mut allowed = 0;
        let mut blocked = false;
        for _ in 0..50 {
            match super::check_strict_limit(unique_ip) {
                Ok(()) => allowed += 1,
                Err(_) => blocked = true,
            }
            if blocked {
                break;
            }
        }
        assert!(
            blocked,
            "strict bucket should eventually block real IP burst"
        );
        assert!(
            allowed <= 6,
            "strict burst is 5, allowed should be <= 6, got {allowed}"
        );
    }

    // 测试辅助函数：绕过环境变量读取，直接指定 trusted_proxy_count。
    fn get_client_ip_with_trusted_and_peer(
        headers: &HeaderMap,
        trusted: usize,
        peer: Option<SocketAddr>,
    ) -> String {
        get_client_ip_internal(headers, trusted, peer)
    }
}

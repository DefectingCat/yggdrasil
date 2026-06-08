#![allow(clippy::unused_unit)]

#[cfg(feature = "server")]
use std::sync::Arc;
#[cfg(feature = "server")]
use tower_governor::governor::GovernorConfigBuilder;
#[cfg(feature = "server")]
use tower_governor::GovernorLayer;
#[cfg(feature = "server")]
use tower_governor::key_extractor::SmartIpKeyExtractor;
#[cfg(feature = "server")]
use governor::middleware::NoOpMiddleware;

/// 通用限流配置：每分钟 60 请求
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

/// 严格限流配置：每分钟 10 请求（用于登录、注册等敏感操作）
#[cfg(feature = "server")]
pub fn strict_limit() -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware> {
    let config = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(5)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .unwrap();
    GovernorLayer {
        config: Arc::new(config),
    }
}

/// 上传限流配置：每分钟 20 请求
#[cfg(feature = "server")]
pub fn upload_limit() -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware> {
    let config = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(10)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .unwrap();
    GovernorLayer {
        config: Arc::new(config),
    }
}

//! 编译期注入的构建元信息。
//!
//! 根目录 `build.rs` 在编译期采集 git / rustc / 编译时刻信息,通过
//! `cargo:rustc-env=KEY=VALUE` 注入为编译期环境变量,本模块用 `env!` 宏读取
//! (编译期内联为 `&'static str`,零运行时开销)。
//!
//! 整个模块 gated 在 `server` feature 下:`log_build_info` 用了 `tracing::info!`,
//! 而 `tracing` 是 optional 依赖(仅 server 启用)。常量定义本身两端都能编译,
//! 但当前没有任何前端代码引用它们,gating 掉更稳妥。
//!
//! 字段拆分取舍:
//! - `git_describe`:一句话承载"版本 + 提交数 + hash + 脏标记",人眼定位构建最快,
//!   已内含短 hash,故不再单独存 short_hash。
//! - `git_hash` / `commit_date` 单独拆出,因为脏树时 describe 带 `-dirty` 但
//!   commit_date 仍是上一个提交的,两者分离才能各自准确。
//! - `build_time`:编译时刻(造出这个二进制的时间),与 commit_date 不同——
//!   CI / 本地可能停在同一个 commit 但时间不同。

#![cfg(feature = "server")]

use chrono::{DateTime, Utc};

/// 构建元信息(编译期常量集合)。
pub struct BuildInfo {
    /// Cargo.toml 里的 `version` 字段。
    pub version: &'static str,
    /// `git describe --tags --always --dirty`,例如 `v0.3.0-200-g0ab3340-dirty`。
    pub git_describe: &'static str,
    /// 完整 40 位 commit hash。
    pub git_hash: &'static str,
    /// 提交时间(ISO 8601 strict,带时区偏移)。
    pub commit_date: &'static str,
    /// `rustc --version`,采集编译工具链。
    pub rustc_version: &'static str,
    /// 编译时刻(Unix 秒),运行时由 chrono 解析回 RFC3339。
    pub build_time: &'static str,
}

/// 全局唯一的构建信息实例。
pub static BUILD_INFO: BuildInfo = BuildInfo {
    version: env!("CARGO_PKG_VERSION"),
    git_describe: env!("YGG_BUILD_GIT_DESCRIBE"),
    git_hash: env!("YGG_BUILD_GIT_HASH"),
    commit_date: env!("YGG_BUILD_GIT_COMMIT_DATE"),
    rustc_version: env!("YGG_BUILD_RUSTC_VERSION"),
    build_time: env!("YGG_BUILD_TIME"),
};

/// 打印构建信息。在 `main()` tracing 初始化之后调用。
///
/// 拆成多条 `info!` 而非一条长串:`RUST_LOG=info` 下每条日志带文件名/行号前缀,
/// 多行更易读,也方便按字段 grep。
pub fn log_build_info() {
    // build_time 存的是 Unix 秒(build.rs 不引 chrono),这里解析回 RFC3339。
    let built_at = BUILD_INFO
        .build_time
        .parse::<i64>()
        .ok()
        .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0))
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| BUILD_INFO.build_time.to_string());

    tracing::info!(
        "build: version={} git={}",
        BUILD_INFO.version,
        BUILD_INFO.git_describe
    );
    tracing::info!(
        "build: commit={} date={}",
        BUILD_INFO.git_hash,
        BUILD_INFO.commit_date
    );
    tracing::info!("build: rustc={}", BUILD_INFO.rustc_version);
    tracing::info!("build: built_at={}", built_at);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_info_fields_are_populated() {
        // build.rs 成功采集 git 信息时,这些字段不应是降级值 "unknown"。
        // (tarball 构建 / 无 git 环境下会跳过——仅作软断言。)
        assert!(!BUILD_INFO.version.is_empty());
        // git_describe 在仓库内一定非空;git 不可用时是 "unknown"。
        assert!(!BUILD_INFO.git_describe.is_empty());
    }

    #[test]
    fn build_time_parses_as_unix_seconds() {
        // build.rs 存的是 Unix 秒,运行时应能解析回时间戳。
        let parsed = BUILD_INFO.build_time.parse::<i64>();
        assert!(
            parsed.is_ok(),
            "build_time not a unix second: {}",
            BUILD_INFO.build_time
        );
        assert!(
            parsed.unwrap() > 1_600_000_000,
            "build_time implausibly old"
        );
    }

    #[test]
    fn log_build_info_does_not_panic() {
        // 无 subscriber 时 tracing::info! 是 no-op,但能确认整个函数跑通。
        log_build_info();
    }
}

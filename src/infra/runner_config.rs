use serde::{Deserialize, Serialize};
use std::env;
use std::sync::LazyLock;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ResourceLimits {
    pub cpu_cores: f64,
    pub memory_mb: u64,
    pub timeout_secs: u64,
    pub output_bytes: u64,
    pub allow_network: bool,
}

pub struct RunnerConfig {
    pub max_cpu_cores: f64,
    pub max_memory_mb: u64,
    pub max_timeout_secs: u64,
    pub max_output_bytes: u64,
    pub max_source_bytes: u64,
    pub allow_network: bool,
    pub max_concurrent: usize,
    pub queue_timeout_secs: u64,
    pub task_ttl_secs: u64,
    pub docker_socket_path: String,
    pub languages: Vec<String>,
}

pub static RUNNER_CONFIG: LazyLock<RunnerConfig> = LazyLock::new(|| {
    let languages_str = env::var("CODE_RUNNER_LANGUAGES").unwrap_or_else(|_| "python,node".to_string());
    let languages = languages_str.split(',').map(|s| s.trim().to_lowercase()).collect();

    RunnerConfig {
        max_cpu_cores: env::var("CODE_RUNNER_MAX_CPU_CORES").ok().and_then(|v| v.parse().ok()).unwrap_or(2.0),
        max_memory_mb: env::var("CODE_RUNNER_MAX_MEMORY_MB").ok().and_then(|v| v.parse().ok()).unwrap_or(1024),
        max_timeout_secs: env::var("CODE_RUNNER_MAX_TIMEOUT_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(30),
        max_output_bytes: env::var("CODE_RUNNER_MAX_OUTPUT_BYTES").ok().and_then(|v| v.parse().ok()).unwrap_or(1048576),
        max_source_bytes: env::var("CODE_RUNNER_MAX_SOURCE_BYTES").ok().and_then(|v| v.parse().ok()).unwrap_or(65536),
        allow_network: env::var("CODE_RUNNER_ALLOW_NETWORK").ok().map(|v| v == "true" || v == "1" || v == "yes").unwrap_or(false),
        max_concurrent: env::var("CODE_RUNNER_MAX_CONCURRENT").ok().and_then(|v| v.parse().ok()).unwrap_or(4),
        queue_timeout_secs: env::var("CODE_RUNNER_QUEUE_TIMEOUT_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(30),
        task_ttl_secs: env::var("CODE_RUNNER_TASK_TTL_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(300),
        docker_socket_path: env::var("DOCKER_SOCKET_PATH").unwrap_or_else(|_| "/var/run/docker.sock".to_string()),
        languages,
    }
});

pub fn clamp_limits(merged: ResourceLimits, lang_allows_network: bool) -> ResourceLimits {
    let config = &*RUNNER_CONFIG;
    ResourceLimits {
        cpu_cores: merged.cpu_cores.clamp(0.1, config.max_cpu_cores),
        memory_mb: merged.memory_mb.clamp(16, config.max_memory_mb),
        timeout_secs: merged.timeout_secs.clamp(1, config.max_timeout_secs),
        output_bytes: merged.output_bytes.min(config.max_output_bytes),
        allow_network: merged.allow_network && config.allow_network && lang_allows_network,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_limits() {
        let raw = ResourceLimits {
            cpu_cores: 5.0,
            memory_mb: 4096,
            timeout_secs: 120,
            output_bytes: 9999999,
            allow_network: true,
        };
        let clamped = clamp_limits(raw, true);
        // 假设运行的是默认的 RUNNER_CONFIG（allow_network 默认为 false，除非 env 改了）
        assert!(clamped.cpu_cores <= 2.0);
        assert!(clamped.memory_mb <= 1024);
        assert!(clamped.timeout_secs <= 30);
        assert!(clamped.output_bytes <= 1048576);
        assert_eq!(clamped.allow_network, false); // 此时全局 allow_network=false
    }
}

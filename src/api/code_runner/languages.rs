//! 语言注册表与围栏代码块 info string 解析。
//!
//! 支持的语言通过 [`LANGUAGES`] 注册（镜像名、运行命令、默认资源限制、是否允许网络）。
//! [`parse_fence_info`] 解析 markdown 围栏代码块的信息串，识别 `runnable` 标记与
//! 可选的 JSON 资源覆盖（如 `python runnable {"timeout_secs":10}`）。
//!
//! 实际可用语言还受 `RUNNER_CONFIG.languages`（环境变量 `CODE_RUNNER_LANGUAGES`）双重约束：
//! 既要在注册表中存在，也要在运维白名单中启用，才视为支持（[`is_supported_lang`]）。

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::infra::runner_config::{ResourceLimits, RUNNER_CONFIG};

/// 单个语言的运行定义。
pub struct LanguageDef {
    pub name: String,
    /// 容器镜像（task 12 的 docker build 产出，如 `yggdrasil-runner-python:latest`）。
    pub image: String,
    /// 容器内执行命令（源码会注入到 `/code/main.{ext}`）。
    pub run_cmd: String,
    pub extension: String,
    pub default_limits: ResourceLimits,
    /// 该语言本身是否允许网络（与全局/请求级 allow_network 取与）。
    pub allow_network: bool,
}

/// 内置语言注册表。新增语言时在此 `insert`，并在 `.env.example` / `CODE_RUNNER_LANGUAGES`
/// 中默认启用。
pub static LANGUAGES: LazyLock<HashMap<String, LanguageDef>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    m.insert(
        "python".to_string(),
        LanguageDef {
            name: "python".to_string(),
            image: "yggdrasil-runner-python:latest".to_string(),
            run_cmd: "python /code/main.py".to_string(),
            extension: "py".to_string(),
            default_limits: ResourceLimits {
                cpu_cores: 1.0,
                memory_mb: 256,
                timeout_secs: 5,
                output_bytes: 1_048_576,
                allow_network: false,
            },
            allow_network: false,
        },
    );

    m.insert(
        "node".to_string(),
        LanguageDef {
            name: "node".to_string(),
            image: "yggdrasil-runner-node:latest".to_string(),
            run_cmd: "node /code/main.js".to_string(),
            extension: "js".to_string(),
            default_limits: ResourceLimits {
                cpu_cores: 1.0,
                memory_mb: 256,
                timeout_secs: 5,
                output_bytes: 1_048_576,
                allow_network: false,
            },
            allow_network: false,
        },
    );

    m
});

/// 同时存在于注册表与运维白名单 (`RUNNER_CONFIG.languages`) 才视为支持。
pub fn is_supported_lang(lang: &str) -> bool {
    let clean = lang.trim().to_lowercase();
    RUNNER_CONFIG.languages.iter().any(|l| l == &clean) && LANGUAGES.contains_key(&clean)
}

/// 解析围栏代码块的 info string。
///
/// 格式：`<lang> [runnable|run] [ {<ResourceLimits JSON>} ]`
///
/// 返回 `(lang, runnable, overrides)`。未知 token 静默忽略；JSON 解析失败时 overrides 为 None。
pub fn parse_fence_info(info: &str) -> (String, bool, Option<ResourceLimits>) {
    let tokens: Vec<&str> = info.split_whitespace().collect();
    if tokens.is_empty() {
        return ("".to_string(), false, None);
    }
    let lang = tokens[0].trim().to_lowercase();
    let mut runnable = false;
    let mut overrides = None;

    if tokens.len() > 1 {
        for &tok in &tokens[1..] {
            if tok == "runnable" || tok == "run" {
                runnable = true;
            } else if tok.starts_with('{') {
                if let Ok(limits) = serde_json::from_str::<ResourceLimits>(tok) {
                    overrides = Some(limits);
                }
            }
        }
    }

    (lang, runnable, overrides)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn parse_fence_info_runnable_marker() {
        let (lang, runnable, overrides) = parse_fence_info("python runnable");
        assert_eq!(lang, "python");
        assert!(runnable);
        assert!(overrides.is_none());
    }

    #[test]
    fn parse_fence_info_run_alias() {
        let (lang, runnable, _) = parse_fence_info("node run");
        assert_eq!(lang, "node");
        assert!(runnable);
    }

    #[test]
    fn parse_fence_info_with_overrides() {
        let (lang, runnable, overrides) =
            parse_fence_info(r#"node runnable {"timeout_secs":10,"memory_mb":512,"allow_network":true,"cpu_cores":1.0,"output_bytes":1024}"#);
        assert_eq!(lang, "node");
        assert!(runnable);
        let limits = overrides.unwrap();
        assert_eq!(limits.timeout_secs, 10);
        assert_eq!(limits.memory_mb, 512);
        assert!(limits.allow_network);
    }

    #[test]
    fn parse_fence_info_not_runnable() {
        let (lang, runnable, _) = parse_fence_info("rust");
        assert_eq!(lang, "rust");
        assert!(!runnable);
    }

    #[test]
    fn parse_fence_info_empty() {
        let (lang, runnable, overrides) = parse_fence_info("");
        assert_eq!(lang, "");
        assert!(!runnable);
        assert!(overrides.is_none());
    }

    #[test]
    fn parse_fence_info_case_insensitive_lang() {
        let (lang, _, _) = parse_fence_info("PYTHON runnable");
        assert_eq!(lang, "python");
    }

    #[test]
    fn parse_fence_info_malformed_json_yields_none() {
        let (lang, runnable, overrides) = parse_fence_info(r#"python runnable {not valid json}"#);
        assert_eq!(lang, "python");
        assert!(runnable);
        assert!(overrides.is_none(), "malformed JSON should yield None");
    }

    #[test]
    fn is_supported_lang_default_whitelist() {
        // 默认 CODE_RUNNER_LANGUAGES=python,node
        assert!(is_supported_lang("python"));
        assert!(is_supported_lang("node"));
        assert!(!is_supported_lang("rust"));
        assert!(!is_supported_lang(""));
    }

    #[test]
    fn is_supported_lang_case_and_whitespace_insensitive() {
        assert!(is_supported_lang(" Python "));
        assert!(is_supported_lang("NODE"));
    }
}

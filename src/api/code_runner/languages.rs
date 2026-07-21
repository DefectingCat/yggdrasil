//! 语言注册表与围栏代码块 info string 解析。
//!
//! 支持的语言通过 [`LANGUAGES`] 注册（镜像名、运行命令、默认资源限制、是否允许网络）。
//! [`parse_fence_info`] 解析 markdown 围栏代码块的信息串，识别 `runnable` 标记与
//! 可选的 JSON 资源覆盖（如 `python runnable {"timeout_secs":10}`）。
//!
//! 实际可用语言默认即注册表里的全部；若设置了 `CODE_RUNNER_LANGUAGES`
//! 环境变量，则进一步收窄到该白名单内（[`is_supported_lang`]）。
//!
//! **别名归一化**：[`normalize_lang`] 把 `js`/`javascript` 归一为 `node`、
//! `rs` 归一为 `rust`、`ts`/`typescript` 归一为 `bun`。归一化在 [`parse_fence_info`]
//! 与 [`is_supported_lang`] 内完成，markdown 渲染期就把别名换成 canonical key，
//! 下游 `data-lang` / `ExecRequest.language` / `LANGUAGES.get` 只见到 canonical
//! 名（`python`/`node`/`go`/`rust`/`bun`），不再出现别名，避免查表落空。

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::infra::runner_config::{ResourceLimits, RUNNER_CONFIG};

/// 单个语言的运行定义。语言名即 [`LANGUAGES`] 的 key，不再冗余存字段。
pub struct LanguageDef {
    /// 容器镜像（task 12 的 docker build 产出，如 `yggdrasil-runner-python:latest`）。
    pub image: String,
    /// 容器内执行命令（源码会注入到 `/code/main.{ext}`）。
    pub run_cmd: String,
    pub extension: String,
    pub default_limits: ResourceLimits,
    /// 该语言本身是否允许网络（与全局/请求级 allow_network 取与）。
    pub allow_network: bool,
}

/// 内置语言注册表。新增语言时在此 `insert` 即可默认启用；
/// 若运维需要收窄，设置 `CODE_RUNNER_LANGUAGES` 为逗号分隔列表。
pub static LANGUAGES: LazyLock<HashMap<String, LanguageDef>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    m.insert(
        "python".to_string(),
        LanguageDef {
            image: "yggdrasil-runner-python:latest".to_string(),
            // -u (unbuffered)：强制 stdout/stderr 行刷新。
            // 容器 attach 用 pipe（非 TTY），Python 默认对 pipe 做块缓冲（4KB），
            // 导致流式输出失效——print 的内容攒在缓冲区，进程退出才一次性刷出。
            // -u 等价于 PYTHONUNBUFFERED=1，让每行 print 立即写出。
            run_cmd: "python -u /code/main.py".to_string(),
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

    // 编译型语言：go run 是单条命令（内部编译 + 运行），可直接作为 run_cmd。
    // 只读根文件系统下 $HOME/.cache 不可写，镜像已把 GOCACHE/GOTMPDIR/GOPATH
    // 重定向到可写的 /tmp tmpfs。编译冷启动比解释型慢，timeout 提到 10s。
    m.insert(
        "go".to_string(),
        LanguageDef {
            image: "yggdrasil-runner-go:latest".to_string(),
            run_cmd: "go run /code/main.go".to_string(),
            extension: "go".to_string(),
            default_limits: ResourceLimits {
                cpu_cores: 1.0,
                memory_mb: 256,
                timeout_secs: 10,
                output_bytes: 1_048_576,
                allow_network: false,
            },
            allow_network: false,
        },
    );

    // rustc 编译 + 运行是两步，但 docker.rs 注入脚本用 exec 执行 run_cmd，
    // exec 替换 shell 进程后 "A && B" 后半段不执行，故镜像内置 run-rust.sh wrapper。
    // rustc 内存开销大、编译慢，memory 提到 512MB、timeout 提到 15s。
    m.insert(
        "rust".to_string(),
        LanguageDef {
            image: "yggdrasil-runner-rust:latest".to_string(),
            run_cmd: "/usr/local/bin/run-rust.sh".to_string(),
            extension: "rs".to_string(),
            default_limits: ResourceLimits {
                cpu_cores: 1.0,
                memory_mb: 512,
                timeout_secs: 15,
                output_bytes: 1_048_576,
                allow_network: false,
            },
            allow_network: false,
        },
    );

    // bun：原生 TypeScript 运行器（无需 tsc 预编译）。`ts`/`typescript` 别名
    // 归一到 `bun`（见 LANG_ALIASES）。bun 镜像用官方安装脚本装在 alpine 上，
    // 单步 `bun run /code/main.ts` 直接执行，无需 wrapper。
    // 默认资源与 node 对齐：JS 运行时开销相近，256MB/5s 足够。
    m.insert(
        "bun".to_string(),
        LanguageDef {
            image: "yggdrasil-runner-bun:latest".to_string(),
            run_cmd: "bun run /code/main.ts".to_string(),
            extension: "ts".to_string(),
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

/// 语言别名 → canonical key 映射。
///
/// canonical key（`python`/`node`/`go`/`rust`/`bun`）本身不在表里——它们原样
/// 通过。仅收录读者/作者习惯的简写：`js`/`javascript`→`node`、`rs`→`rust`、
/// `ts`/`typescript`→`bun`。比较大小写不敏感（见 [`normalize_lang`]）。
const LANG_ALIASES: &[(&str, &str)] = &[
    ("js", "node"),
    ("javascript", "node"),
    ("rs", "rust"),
    ("ts", "bun"),
    ("typescript", "bun"),
];

/// 把语言标识归一化为 `LANGUAGES` 注册表里的 canonical key。
///
/// 步骤：trim → lowercase → 查 [`LANG_ALIASES`]，命中返回映射值，未命中返回
/// trim+lowercase 后的原值（保证 canonical 名与未注册字符串都能被调用方处理）。
///
/// 归一化是 [`is_supported_lang`] / [`parse_fence_info`] 的前置步骤，也是
/// `execute.rs` 在 `LANGUAGES.get` 前必须调用的——否则用别名查表会落空。
pub fn normalize_lang(lang: &str) -> String {
    let clean = lang.trim().to_lowercase();
    for &(from, to) in LANG_ALIASES {
        if clean == from {
            return to.to_string();
        }
    }
    clean
}

/// 是否支持该语言：先 [`normalize_lang`] 归一化，再查 `LANGUAGES` 注册表。
/// 若设置了 `CODE_RUNNER_LANGUAGES`，还需同时在该白名单内（用于收窄可用语言，
/// 白名单用 canonical key 比较——运维写 `node` 而非 `js`）；未设置则全部放行。
pub fn is_supported_lang(lang: &str) -> bool {
    let canonical = normalize_lang(lang);
    LANGUAGES.contains_key(&canonical)
        && RUNNER_CONFIG
            .languages
            .as_ref()
            .is_none_or(|list| list.iter().any(|l| l == &canonical))
}

/// 解析围栏代码块的 info string。
///
/// 格式：`<lang> [runnable|run] [ {<ResourceLimits JSON>} ]`
///
/// 返回 `(lang, runnable, overrides)`。`lang` 已经过 [`normalize_lang`] 归一化
/// （`js`→`node`、`ts`→`bun` 等），下游 markdown 渲染 / ExecRequest.language /
/// `LANGUAGES.get` 都拿到 canonical key。未知 token 静默忽略；JSON 解析失败时
/// overrides 为 None。
pub fn parse_fence_info(info: &str) -> (String, bool, Option<ResourceLimits>) {
    let tokens: Vec<&str> = info.split_whitespace().collect();
    if tokens.is_empty() {
        return ("".to_string(), false, None);
    }
    let lang = normalize_lang(tokens[0]);
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
        let (lang, runnable, overrides) = parse_fence_info(
            r#"node runnable {"timeout_secs":10,"memory_mb":512,"allow_network":true,"cpu_cores":1.0,"output_bytes":1024}"#,
        );
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
    fn is_supported_lang_default_all_open() {
        // 默认未设 CODE_RUNNER_LANGUAGES：注册表里的语言全部放行。
        assert!(is_supported_lang("python"));
        assert!(is_supported_lang("node"));
        assert!(is_supported_lang("go"));
        assert!(is_supported_lang("rust"));
        // 未注册的语言仍不支持。
        assert!(!is_supported_lang("ruby"));
        assert!(!is_supported_lang(""));
    }

    #[test]
    fn is_supported_lang_case_and_whitespace_insensitive() {
        assert!(is_supported_lang(" Python "));
        assert!(is_supported_lang("NODE"));
    }

    // ---- 别名归一化（normalize_lang / parse_fence_info / is_supported_lang 三处协同）----

    #[test]
    fn normalize_lang_canonical_passthrough() {
        // canonical key 原样通过（仅 trim+lowercase）。
        assert_eq!(normalize_lang("python"), "python");
        assert_eq!(normalize_lang("node"), "node");
        assert_eq!(normalize_lang("go"), "go");
        assert_eq!(normalize_lang("rust"), "rust");
        assert_eq!(normalize_lang("bun"), "bun");
    }

    #[test]
    fn normalize_lang_aliases() {
        // 别名 → canonical。
        assert_eq!(normalize_lang("js"), "node");
        assert_eq!(normalize_lang("javascript"), "node");
        assert_eq!(normalize_lang("rs"), "rust");
        assert_eq!(normalize_lang("ts"), "bun");
        assert_eq!(normalize_lang("typescript"), "bun");
    }

    #[test]
    fn normalize_lang_case_and_whitespace_insensitive() {
        // 输入容忍：大小写、首尾空白。匹配 is_supported_lang 的既有契约。
        assert_eq!(normalize_lang("  JS  "), "node");
        assert_eq!(normalize_lang("JavaScript"), "node");
        assert_eq!(normalize_lang("\tRS\t"), "rust");
        assert_eq!(normalize_lang("TypeScript"), "bun");
    }

    #[test]
    fn normalize_lang_unregistered_passthrough() {
        // 未在别名表、未在注册表的语言原样返回（trim+lowercase 后），
        // 由调用方决定是否拒绝。normalize_lang 不做存在性判断。
        assert_eq!(normalize_lang("ruby"), "ruby");
        assert_eq!(normalize_lang("Brainfuck"), "brainfuck");
        assert_eq!(normalize_lang(""), "");
    }

    #[test]
    fn is_supported_lang_accepts_aliases() {
        // 别名经 normalize_lang 归一后命中注册表，应放行。
        assert!(is_supported_lang("js"));
        assert!(is_supported_lang("javascript"));
        assert!(is_supported_lang("rs"));
        assert!(is_supported_lang("ts"));
        assert!(is_supported_lang("typescript"));
    }

    #[test]
    fn is_supported_lang_accepts_bun_canonical() {
        // bun 作为新增 canonical key，默认放行。
        assert!(is_supported_lang("bun"));
    }

    #[test]
    fn parse_fence_info_normalizes_alias_to_canonical() {
        // 关键契约：parse_fence_info 返回的 lang 是 canonical key，
        // 下游 markdown.rs 的 data-lang / class="language-{lang}" 直接用此值。
        let (lang, runnable, _) = parse_fence_info("js runnable");
        assert_eq!(lang, "node");
        assert!(runnable);

        let (lang, _, _) = parse_fence_info("typescript runnable");
        assert_eq!(lang, "bun");

        let (lang, _, _) = parse_fence_info("rs runnable");
        assert_eq!(lang, "rust");

        // 大小写不敏感。
        let (lang, _, _) = parse_fence_info("JavaScript runnable");
        assert_eq!(lang, "node");
    }

    #[test]
    fn parse_fence_info_bun_canonical_runs_as_ts() {
        // bun 自身是 canonical，runnable 块以 bun 执行。
        let (lang, runnable, _) = parse_fence_info("bun runnable");
        assert_eq!(lang, "bun");
        assert!(runnable);
    }
}

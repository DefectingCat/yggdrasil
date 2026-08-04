//! 代码运行器 MCP 工具：在容器沙箱内执行代码，同步返回完整输出。
//!
//! 与 `src/api/code_runner/execute.rs::start_exec` 的执行链一致（语言白名单 →
//! 源码大小 → 信号量限并发 → clamp_limits → run_in_container），但 **同步返回**
//! 完整 stdout/stderr，不引入 task_id / 轮询 / SSE 机制——MCP 工具返回单一结果。
//!
//! 鉴权与限流：MCP 走 bearer token → admin 作用域。admin 跳过 IP 速率限制
//! （与 web 的 `check_rate_limit_for_user` admin 放行一致），但仍受并发槽、
//! 资源钳制与源码大小校验约束。Docker daemon 不可用时返回明确错误（与
//! `get_docker()` 的 NotFound 脱敏路径一致）。

#![cfg(feature = "server")]

use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, TextContent};
use rmcp::{schemars, tool, tool_router, ErrorData as McpError};

use super::common::require_admin;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::api::code_runner::execute::RUNNER_SEMAPHORE;
use crate::api::code_runner::languages::{is_supported_lang, normalize_lang, LANGUAGES};
use crate::infra::docker::run_in_container;
use crate::infra::runner_config::{clamp_limits, RUNNER_CONFIG};

/// `run_code` 入参。
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RunCodeParams {
    /// 语言标识（支持别名归一化：js/javascript→node、ts/typescript→bun、rs→rust；
    /// canonical: python/node/go/rust/bun）。
    pub language: String,
    /// 源代码（受 `CODE_RUNNER_MAX_SOURCE_BYTES` 限制，默认 64KB）。
    pub source: String,
}

/// `run_code` 返回的执行结果。
#[derive(Debug, Serialize)]
struct RunResult {
    /// 执行终态：success / error / oom / timeout / unavailable。
    status: &'static str,
    /// 进程退出码（容器未跑完 / 超时 / daemon 不可用时为 null）。
    exit_code: Option<i64>,
    /// 标准输出（已按 output_bytes 上限截断）。
    stdout: String,
    /// 标准错误 / 失败原因描述（已按 output_bytes 上限截断）。
    stderr: String,
    /// 执行耗时（毫秒）。
    duration_ms: u64,
    /// 归一化后的 canonical 语言 key（如 python/node/go/rust/bun）。
    language: String,
}

#[tool_router(router = runner_router, vis = "pub")]
impl crate::mcp::server::YggMcpServer {
    /// 在容器沙箱内执行代码并返回输出。要求 admin 作用域。
    #[tool(
        description = "在 Docker 沙箱内执行代码（支持 python/node/go/rust/bun），返回 stdout/stderr。需要 admin 作用域。"
    )]
    async fn run_code(
        &self,
        Parameters(RunCodeParams { language, source }): Parameters<RunCodeParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        require_admin(&parts, "run_code")?;

        // 1. 语言白名单（与 validate_exec_request 一致；is_supported_lang 内含归一化）。
        if !is_supported_lang(&language) {
            return Err(McpError::invalid_request(
                "unsupported language: use one of python/node/go/rust/bun (js/rs/ts aliases accepted)",
                None,
            ));
        }

        // 2. 源码大小限制。
        if source.len() > RUNNER_CONFIG.max_source_bytes as usize {
            return Err(McpError::invalid_request(
                format!(
                    "source too large: {} bytes > limit {}",
                    source.len(),
                    RUNNER_CONFIG.max_source_bytes
                ),
                None,
            ));
        }

        let result = execute_in_container(&language, &source)
            .await
            .map_err(|e| McpError::internal_error(format!("code execution failed: {e}"), None))?;

        let text = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("encode failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::Text(
            TextContent::new(text),
        )]))
    }
}

/// 执行一次容器内代码运行（同步收集输出）。
///
/// 镜像 start_exec 的后台 spawn 体：排队信号量 → 归一化语言 → clamp_limits →
/// run_in_container，但把结果直接返回而非写入 EXEC_TASKS。
async fn execute_in_container(language: &str, source: &str) -> Result<RunResult, String> {
    let lang_key = normalize_lang(language);

    let lang_def = LANGUAGES
        .get(&lang_key)
        .ok_or_else(|| format!("language not registered: {lang_key}"))?;

    // 排队等待可用容器槽（与 start_exec 一致的 queue_timeout_secs）。
    let ticket = tokio::time::timeout(
        Duration::from_secs(RUNNER_CONFIG.queue_timeout_secs),
        RUNNER_SEMAPHORE.acquire(),
    )
    .await
    .map_err(|_| "container queue timeout: too many concurrent executions".to_string())?
    .map_err(|e| format!("semaphore acquire: {e}"))?;

    let final_limits = clamp_limits(lang_def.default_limits.clone(), lang_def.allow_network);

    let start_time = chrono::Utc::now();
    let res = run_in_container(
        &lang_def.image,
        &lang_def.run_cmd,
        source,
        &lang_def.extension,
        final_limits,
        lang_def.cache_volume.as_ref(),
    )
    .await;
    let duration_ms = (chrono::Utc::now() - start_time).num_milliseconds().max(0) as u64;

    drop(ticket); // 显式释放信号量

    match res {
        Ok((exit_code, stdout, stderr, oom_killed)) => {
            let status = if oom_killed {
                "oom"
            } else if exit_code == Some(0) {
                "success"
            } else {
                "error"
            };
            Ok(RunResult {
                status,
                exit_code,
                stdout,
                stderr,
                duration_ms,
                language: lang_key,
            })
        }
        Err(e) => {
            // 脱敏与 start_exec 的失败分支一致：日志记详情，对外给通用消息。
            let s = e.to_string();
            let is_timeout = s.contains("TimedOut");
            tracing::error!(error = ?e, "MCP container execution failed");
            // bollard IOError{NotFound} = Docker daemon 不可用 → 明确的 unavailable。
            let is_daemon_down = s.contains("Docker daemon") || s.contains("NotFound");
            let status = if is_daemon_down {
                "unavailable"
            } else if is_timeout {
                "timeout"
            } else {
                "error"
            };
            let stderr = if is_daemon_down {
                "code runner unavailable: Docker daemon not running".to_string()
            } else if is_timeout {
                "execution timed out".to_string()
            } else {
                "code runner temporarily unavailable".to_string()
            };
            Ok(RunResult {
                status,
                exit_code: None,
                stdout: String::new(),
                stderr,
                duration_ms,
                language: lang_key,
            })
        }
    }
}

//! 代码执行 server functions：StartExec / GetExecResult。
//!
//! StartExec 流程：速率限制 → 语言白名单 → 源码大小校验 → 入队（DashMap）
//! → spawn 后台 task（信号量限并发 + clamp_limits + run_in_container）。
//! 返回 task_id 供前端轮询。GetExecResult 读取任务条目。
//!
//! 错误脱敏：匿名可见错误（不支持的语言、超限、限流）返回中文消息；系统内部
//! 异常（容器拉起失败等）记服务端日志，对前端返回统一「系统暂时不可用」。
//!
//! 双目标可见性：本模块**不** cfg-gate（server function 需对 WASM 可见以便客户端
//! 调用），但所有 server-only 的 `use` 与全局静态量都单独 gate，使 WASM 侧仅保留
//! 函数签名（body 被 server 宏剥离）。与 posts 模块的约定一致。

// 与 posts / settings 模块一致：Dioxus `#[server]` 宏触发 deprecated/unit 提示，按项目惯例放行。
#![allow(clippy::unused_unit, deprecated)]

use dioxus::prelude::*;

// 共享数据类型在函数签名中出现，双目标可见，不 gate。
use crate::api::code_runner::{ExecRequest, ExecTask};
// ExecResult/ExecStatus 仅在 server function body（被宏剥离到 WASM 之外）内使用。
#[cfg(feature = "server")]
use crate::api::code_runner::{ExecResult, ExecStatus};

// server-only 辅助模块与依赖：仅在 server function body（被宏剥离到 WASM 之外）内使用。
#[cfg(feature = "server")]
use crate::api::auth::get_current_admin_user;
#[cfg(feature = "server")]
use crate::api::code_runner::languages::{is_supported_lang, LANGUAGES};
#[cfg(feature = "server")]
use crate::api::code_runner::progress::{
    gc_old_tasks, insert_task, update_task_result, update_task_stage, StreamEntry, EXEC_STREAMS,
    EXEC_TASKS,
};
#[cfg(feature = "server")]
use crate::api::rate_limit::{check_code_exec_limit, get_client_ip};
#[cfg(feature = "server")]
use crate::infra::docker::{run_in_container, run_in_container_stream};
#[cfg(feature = "server")]
use crate::infra::runner_config::{clamp_limits, RUNNER_CONFIG};
#[cfg(feature = "server")]
use std::sync::{Arc, LazyLock};
#[cfg(feature = "server")]
use std::time::Duration;
#[cfg(feature = "server")]
use tokio::sync::Semaphore;

/// 并发容器控制信号量，限制同时在跑的容器数量（`CODE_RUNNER_MAX_CONCURRENT`）。
#[cfg(feature = "server")]
pub static RUNNER_SEMAPHORE: LazyLock<Arc<Semaphore>> =
    LazyLock::new(|| Arc::new(Semaphore::new(RUNNER_CONFIG.max_concurrent)));

/// 从 FullstackContext 提取客户端 IP（无上下文时退回 "unknown"）。
#[cfg(feature = "server")]
fn client_ip() -> String {
    match dioxus::fullstack::FullstackContext::current() {
        Some(ctx) => {
            let parts = ctx.parts_mut();
            get_client_ip(&parts.headers)
        }
        None => "unknown".to_string(),
    }
}

/// 公共校验逻辑：速率限制 + 语言白名单 + 源码大小。
///
/// [`start_exec`]（轮询路径）与 [`start_exec_stream`]（SSE 路径）共用，
/// 保证两条路径的准入约束完全一致。admin 跳过速率限制。
#[cfg(feature = "server")]
fn validate_exec_request(req: &ExecRequest) -> Result<(), ServerFnError> {
    // 1. 语言白名单
    if !is_supported_lang(&req.language) {
        return Err(ServerFnError::new("不支持该执行语言".to_string()));
    }

    // 2. 源码大小限制
    if req.source.len() > RUNNER_CONFIG.max_source_bytes as usize {
        return Err(ServerFnError::new("源代码过大".to_string()));
    }

    Ok(())
}

/// 速率限制检查（admin 放行）。
#[cfg(feature = "server")]
async fn check_rate_limit_for_user() -> Result<(), ServerFnError> {
    let is_admin = get_current_admin_user().await.is_ok();
    if !is_admin {
        let ip = client_ip();
        if let Err(msg) = check_code_exec_limit(&ip) {
            return Err(ServerFnError::new(msg));
        }
    }
    Ok(())
}

/// 提交一次代码执行请求。
///
/// 同步校验通过后立即返回 task_id，容器在后台执行；结果通过
/// [`get_exec_result`] 轮询查询。语言不支持 / 源码过大 / 触发限流时同步返回错误。
///
/// admin 角色跳过速率限制（便于作者在沙箱调试），但仍受并发槽、
/// 资源钳制与源码大小校验约束。
#[server(StartExec, "/api")]
pub async fn start_exec(req: ExecRequest) -> Result<String, ServerFnError> {
    check_rate_limit_for_user().await?;
    validate_exec_request(&req)?;

    // 生成任务 ID 并入队
    let task_id = uuid::Uuid::new_v4().to_string();
    insert_task(task_id.clone());

    // 顺手回收过期任务（同时清 EXEC_TASKS 和 EXEC_STREAMS）
    gc_old_tasks();

    // 后台执行：信号量限并发 → clamp_limits → run_in_container
    let task_id_clone = task_id.clone();
    let lang_key = req.language.clone();
    tokio::spawn(async move {
        let sem = &*RUNNER_SEMAPHORE;

        // 排队等待可用容器槽
        let ticket = match tokio::time::timeout(
            Duration::from_secs(RUNNER_CONFIG.queue_timeout_secs),
            sem.acquire(),
        )
        .await
        {
            Ok(Ok(t)) => t,
            _ => {
                update_task_stage(&task_id_clone, ExecStatus::Failed, "系统繁忙，排队超时");
                return;
            }
        };

        update_task_stage(&task_id_clone, ExecStatus::Running, "启动容器");

        let lang_def = match LANGUAGES.get(&lang_key) {
            Some(d) => d,
            None => {
                // 理论不可达：start_exec 已校验白名单；防御性兜底。
                update_task_stage(&task_id_clone, ExecStatus::Failed, "语言未注册");
                return;
            }
        };

        // 资源限制合并与钳制
        let base_limits = req
            .overrides
            .unwrap_or_else(|| lang_def.default_limits.clone());
        let final_limits = clamp_limits(base_limits, lang_def.allow_network);

        let start_time = chrono::Utc::now();
        let res = run_in_container(
            &lang_def.image,
            &lang_def.run_cmd,
            &req.source,
            &lang_def.extension,
            final_limits,
        )
        .await;
        let duration_ms = (chrono::Utc::now() - start_time).num_milliseconds().max(0) as u64;

        drop(ticket); // 显式释放信号量

        match res {
            Ok((exit_code, stdout, stderr, oom_killed)) => {
                let status = if oom_killed {
                    ExecStatus::OomKilled
                } else if exit_code == Some(0) {
                    ExecStatus::Success
                } else {
                    ExecStatus::Error
                };
                let exec_res = ExecResult {
                    status: status.clone(),
                    stdout,
                    stderr,
                    exit_code,
                    duration_ms,
                    language: lang_key.clone(),
                };
                update_task_result(&task_id_clone, status, exec_res);
            }
            Err(e) => {
                // 系统内部异常脱敏：日志记详情，前端只见通用消息。
                let s = e.to_string();
                let is_timeout = s.contains("TimedOut");
                tracing::error!(error = ?e, task_id = %task_id_clone, "container execution failed");
                let status = if is_timeout {
                    ExecStatus::Timeout
                } else {
                    ExecStatus::Failed
                };
                let exec_res = ExecResult {
                    status: status.clone(),
                    stdout: String::new(),
                    stderr: if is_timeout {
                        "执行超时".to_string()
                    } else {
                        "系统暂时不可用".to_string()
                    },
                    exit_code: None,
                    duration_ms,
                    language: lang_key.clone(),
                };
                update_task_result(&task_id_clone, status, exec_res);
            }
        }
    });

    Ok(task_id)
}

/// 提交一次流式代码执行请求。
///
/// 校验链与 [`start_exec`] 完全一致（速率限制 + 白名单 + 大小），返回 task_id。
/// 前端拿到 task_id 后用 EventSource 连 `GET /api/exec/stream?task_id=X`，
/// SSE handler 从 `EXEC_STREAMS` 取走 receiver 做流式输出。
///
/// 后台 spawn 与 `start_exec` 并行写两处：
/// - `run_in_container_stream` 推 chunk 到 SSE（流式路径）
/// - `update_task_result` 写 `EXEC_TASKS`（轮询兜底路径，SSE 不可用或 Tiptap 编辑器用）
#[server(StartExecStream, "/api")]
pub async fn start_exec_stream(req: ExecRequest) -> Result<String, ServerFnError> {
    check_rate_limit_for_user().await?;
    validate_exec_request(&req)?;

    let task_id = uuid::Uuid::new_v4().to_string();
    // EXEC_TASKS 给轮询兜底路径（get_exec_result / Tiptap 编辑器 run-code 闭包）。
    insert_task(task_id.clone());

    // EXEC_STREAMS 给 SSE 流式路径：创建 channel，rx 存表等前端取，tx 进后台 task。
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    EXEC_STREAMS.insert(
        task_id.clone(),
        StreamEntry {
            rx,
            created_at: chrono::Utc::now(),
        },
    );

    gc_old_tasks();

    let task_id_clone = task_id.clone();
    let lang_key = req.language.clone();
    tokio::spawn(async move {
        let sem = &*RUNNER_SEMAPHORE;

        let ticket = match tokio::time::timeout(
            Duration::from_secs(RUNNER_CONFIG.queue_timeout_secs),
            sem.acquire(),
        )
        .await
        {
            Ok(Ok(t)) => t,
            _ => {
                update_task_stage(&task_id_clone, ExecStatus::Failed, "系统繁忙，排队超时");
                return;
            }
        };

        update_task_stage(&task_id_clone, ExecStatus::Running, "启动容器");

        let lang_def = match LANGUAGES.get(&lang_key) {
            Some(d) => d,
            None => {
                update_task_stage(&task_id_clone, ExecStatus::Failed, "语言未注册");
                return;
            }
        };

        let base_limits = req
            .overrides
            .unwrap_or_else(|| lang_def.default_limits.clone());
        let final_limits = clamp_limits(base_limits, lang_def.allow_network);

        let start_time = chrono::Utc::now();
        let res = run_in_container_stream(
            &lang_def.image,
            &lang_def.run_cmd,
            &req.source,
            &lang_def.extension,
            final_limits,
            tx,
        )
        .await;
        let duration_ms = (chrono::Utc::now() - start_time).num_milliseconds().max(0) as u64;

        drop(ticket); // 显式释放信号量

        match res {
            Ok((exit_code, stdout, stderr, oom_killed, _timed_out)) => {
                let status = if oom_killed {
                    ExecStatus::OomKilled
                } else if exit_code == Some(0) {
                    ExecStatus::Success
                } else {
                    ExecStatus::Error
                };
                let exec_res = ExecResult {
                    status: status.clone(),
                    stdout,
                    stderr,
                    exit_code,
                    duration_ms,
                    language: lang_key.clone(),
                };
                update_task_result(&task_id_clone, status, exec_res);
            }
            Err(e) => {
                let s = e.to_string();
                let is_timeout = s.contains("TimedOut");
                tracing::error!(error = ?e, task_id = %task_id_clone, "container execution failed (stream)");
                let status = if is_timeout {
                    ExecStatus::Timeout
                } else {
                    ExecStatus::Failed
                };
                let exec_res = ExecResult {
                    status: status.clone(),
                    stdout: String::new(),
                    stderr: if is_timeout {
                        "执行超时".to_string()
                    } else {
                        "系统暂时不可用".to_string()
                    },
                    exit_code: None,
                    duration_ms,
                    language: lang_key.clone(),
                };
                update_task_result(&task_id_clone, status, exec_res);
            }
        }
    });

    Ok(task_id)
}

/// 查询任务执行结果（前端轮询）。
#[server(GetExecResult, "/api")]
pub async fn get_exec_result(task_id: String) -> Result<ExecTask, ServerFnError> {
    if let Some(task) = EXEC_TASKS.get(&task_id) {
        Ok(task.clone())
    } else {
        Err(ServerFnError::new("找不到指定的任务".to_string()))
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    fn req(language: &str, source: &str) -> ExecRequest {
        ExecRequest {
            language: language.to_string(),
            source: source.to_string(),
            overrides: None,
        }
    }

    #[test]
    fn validate_accepts_registered_language() {
        // python/node/go/rust 默认注册，应放行。
        for lang in ["python", "node", "go", "rust"] {
            assert!(
                validate_exec_request(&req(lang, "x")).is_ok(),
                "{lang} 应被支持"
            );
        }
    }

    #[test]
    fn validate_language_is_case_and_whitespace_insensitive() {
        // is_supported_lang 内部 trim+lowercase，校验链应透传该容忍。
        assert!(validate_exec_request(&req("Python", "x")).is_ok());
        assert!(validate_exec_request(&req("  RUST  ", "x")).is_ok());
        assert!(validate_exec_request(&req("Go", "x")).is_ok());
    }

    #[test]
    fn validate_rejects_unregistered_language() {
        // 未注册语言 / 命令注入尝试都应在白名单阶段被拒。
        assert!(validate_exec_request(&req("c", "x")).is_err());
        assert!(validate_exec_request(&req("bash", "x")).is_err());
        assert!(validate_exec_request(&req("python2", "x")).is_err());
        assert!(validate_exec_request(&req("", "x")).is_err());
    }

    #[test]
    fn validate_rejects_multi_token_language() {
        // 语言字段不会到达容器层：只有命中 LANGUAGES 的 key 才放行，
        // 否则被拒。任何多 token / 含 shell 元字符的值都因找不到注册项而拒绝。
        assert!(validate_exec_request(&req("python; rm -rf /", "x")).is_err());
        assert!(validate_exec_request(&req("python node", "x")).is_err());
        assert!(validate_exec_request(&req("python$(whoami)", "x")).is_err());
    }

    #[test]
    fn validate_language_tolerates_surrounding_whitespace() {
        // is_supported_lang 内部 trim+lowercase：首尾空白（含换行）被吃掉后等于 "python"。
        // 锁定该契约——语言字段只用于 HashMap 查 key，空白无害。
        assert!(validate_exec_request(&req("python\n", "x")).is_ok());
        assert!(validate_exec_request(&req("\tpython\t", "x")).is_ok());
    }

    #[test]
    fn validate_rejects_source_exceeding_max_bytes() {
        let max = RUNNER_CONFIG.max_source_bytes as usize;
        // 恰好 max 放行，max+1 拒绝——边界精确，防止 off-by-one 让攻击者多塞 1 字节。
        let exactly = "a".repeat(max);
        assert!(
            validate_exec_request(&req("python", &exactly)).is_ok(),
            "源码恰好等于上限应放行"
        );
        let over = "a".repeat(max + 1);
        let err =
            validate_exec_request(&req("python", &over)).expect_err("超出上限应拒绝");
        assert!(err.to_string().contains("过大"), "错误信息应提及大小: {err}");
    }

    #[test]
    fn validate_empty_source_accepted_for_supported_lang() {
        // 空源码不是校验层的职责（容器层处理），这里只验语言/大小。
        // 锁定该契约：空源码 + 合法语言 → Ok，避免未来误把空当拒绝。
        assert!(validate_exec_request(&req("python", "")).is_ok());
    }

    #[test]
    fn validate_checks_language_before_size() {
        // 不支持语言 + 巨大源码：应先因语言被拒（而非先报大小）。
        let huge = "a".repeat((RUNNER_CONFIG.max_source_bytes as usize) + 100);
        let err = validate_exec_request(&req("brainfuck", &huge)).unwrap_err();
        assert!(err.to_string().contains("语言"), "应先报语言错误: {err}");
    }
}

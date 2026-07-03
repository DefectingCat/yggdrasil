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
use crate::api::code_runner::{ExecRequest, ExecResult, ExecStatus, ExecTask};

// server-only 辅助模块与依赖：仅在 server function body（被宏剥离到 WASM 之外）内使用。
#[cfg(feature = "server")]
use crate::api::code_runner::languages::{is_supported_lang, LANGUAGES};
#[cfg(feature = "server")]
use crate::api::code_runner::progress::{
    gc_old_tasks, insert_task, update_task_result, update_task_stage, EXEC_TASKS,
};
#[cfg(feature = "server")]
use crate::api::rate_limit::{check_code_exec_limit, get_client_ip};
#[cfg(feature = "server")]
use crate::infra::docker::run_in_container;
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

/// 提交一次代码执行请求。
///
/// 同步校验通过后立即返回 task_id，容器在后台执行；结果通过
/// [`get_exec_result`] 轮询查询。语言不支持 / 源码过大 / 触发限流时同步返回错误。
#[server(StartExec, "/api")]
pub async fn start_exec(req: ExecRequest) -> Result<String, ServerFnError> {
    // 1. 速率限制（双层：每秒突发 + 每日总额）
    let ip = client_ip();
    if let Err(msg) = check_code_exec_limit(&ip) {
        return Err(ServerFnError::new(msg));
    }

    // 2. 语言白名单
    if !is_supported_lang(&req.language) {
        return Err(ServerFnError::new("不支持该执行语言".to_string()));
    }

    // 3. 源码大小限制
    if req.source.len() > RUNNER_CONFIG.max_source_bytes as usize {
        return Err(ServerFnError::new("源代码过大".to_string()));
    }

    // 4. 生成任务 ID 并入队
    let task_id = uuid::Uuid::new_v4().to_string();
    insert_task(task_id.clone());

    // 5. 顺手回收过期任务
    gc_old_tasks();

    // 6. 后台执行
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
        let duration_ms = (chrono::Utc::now() - start_time)
            .num_milliseconds()
            .max(0) as u64;

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

/// 查询任务执行结果（前端轮询）。
#[server(GetExecResult, "/api")]
pub async fn get_exec_result(task_id: String) -> Result<ExecTask, ServerFnError> {
    if let Some(task) = EXEC_TASKS.get(&task_id) {
        Ok(task.clone())
    } else {
        Err(ServerFnError::new("找不到指定的任务".to_string()))
    }
}

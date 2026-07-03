//! CodeRunner 组件实现：源码 + 运行按钮 + 轮询 + 输出区。

use dioxus::prelude::*;

use crate::api::code_runner::execute::{get_exec_result, start_exec};
use crate::api::code_runner::{ExecRequest, ExecStatus};
use crate::infra::runner_config::ResourceLimits;
use crate::utils::time::sleep_ms;

/// 轮询间隔（毫秒）。
const POLL_INTERVAL_MS: u32 = 500;
/// 轮询最大次数兜底，避免任务卡在 Running 状态时无限轮询。
const MAX_POLLS: u32 = 240; // 500ms * 240 = 120s 上限

/// 代码运行器组件。
///
/// Props：
/// - `source`：初始源码（用于展示与提交）。
/// - `language`：语言标识（python / node 等）。
/// - `overrides`：可选资源限制覆盖。
///
/// `mut` 信号仅在 WASM 的 spawn 闭包内被 `.set()`，server 构建会触发 unused_mut，
/// 故按项目惯例加 `cfg_attr` 放行（参见 AGENTS.md「mut bindings needed only on WASM」）。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
pub fn CodeRunner(
    source: String,
    language: String,
    overrides: Option<ResourceLimits>,
) -> Element {
    let mut running = use_signal(|| false);
    let mut stage = use_signal(String::new);
    let mut output = use_signal(String::new);
    let mut exit_info = use_signal(String::new);
    let mut error_msg = use_signal(String::new);

    // 为每个实例生成稳定的容器 id（CodeMirror 容器，由调用方在 WASM 端挂载）。
    // use_id 在 Dioxus 0.7 提供 scope 内稳定的 id；退回 use_hook 保证只算一次。
    let container_id = use_hook(|| {
        format!(
            "code-runner-{}",
            // 简单用一次性随机后缀；非安全场景，避免额外依赖。
            now_pseudo_unique()
        )
    });

    // 提前克隆一份给 run 闭包使用，避免 move 闭包抢走 rsx! 仍要读的 source/language。
    let run_source = source.clone();
    let run_language = language.clone();
    let run_overrides = overrides.clone();

    let run_code = move |_| {
        if running() {
            return;
        }
        running.set(true);
        stage.set("提交中...".to_string());
        output.set(String::new());
        exit_info.set(String::new());
        error_msg.set(String::new());

        let req = ExecRequest {
            language: run_language.clone(),
            source: run_source.clone(),
            overrides: run_overrides.clone(),
        };

        spawn(async move {
            match start_exec(req).await {
                Ok(task_id) => {
                    let mut polls = 0u32;
                    loop {
                        polls += 1;
                        sleep_ms(POLL_INTERVAL_MS).await;
                        match get_exec_result(task_id.clone()).await {
                            Ok(task) => {
                                stage.set(task.stage.clone());
                                let terminal = task.status != ExecStatus::Queued
                                    && task.status != ExecStatus::Running;
                                if terminal {
                                    running.set(false);
                                    if let Some(res) = task.result {
                                        let out = format!(
                                            "Stdout:\n{}\nStderr:\n{}",
                                            res.stdout, res.stderr
                                        );
                                        output.set(out);
                                        exit_info.set(format!(
                                            "耗时: {}ms · 状态: {}",
                                            res.duration_ms,
                                            status_label(&res.status)
                                        ));
                                        if res.status == ExecStatus::Success {
                                            error_msg.set(String::new());
                                        } else {
                                            error_msg
                                                .set(status_label(&res.status));
                                        }
                                    }
                                    break;
                                }
                                if polls >= MAX_POLLS {
                                    running.set(false);
                                    stage.set("查询超时".to_string());
                                    error_msg.set("轮询超时，请重试".to_string());
                                    break;
                                }
                            }
                            Err(_) => {
                                running.set(false);
                                stage.set("结果获取异常".to_string());
                                error_msg.set("结果获取异常".to_string());
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    running.set(false);
                    let msg = e.to_string();
                    stage.set(msg.clone());
                    error_msg.set(msg);
                }
            }
        });
    };

    rsx! {
        div { class: "code-runner-container border border-base-300 rounded-xl overflow-hidden my-4 bg-base-100",
            div { class: "flex justify-between items-center bg-base-200 px-3 py-1 text-sm border-b border-base-300",
                span { class: "font-mono font-bold text-base-content/80", "{language}" }
                button {
                    class: "btn btn-xs btn-primary rounded-lg",
                    disabled: running(),
                    onclick: run_code,
                    if running() {
                        span { class: "loading loading-spinner loading-xs mr-1" }
                        "{stage()}"
                    } else {
                        "Run"
                    }
                }
            }
            // CodeMirror 容器：调用方（阅读器扫描 / 后台试运行）在 WASM 端按此 id 挂载编辑器。
            // 服务端渲染时仅展示源码文本，避免在 SSR 阶段拉起 JS 编辑器。
            div {
                id: "{container_id}",
                class: "code-runner-editor min-h-[100px] font-mono text-sm",
                // 源码以 data 属性承载，供挂载脚本读取初始化（避免 SSR 时把源码当 HTML 解析）。
                "data-source": "{source}",
            }
            if !output().is_empty() {
                div { class: "output-area bg-neutral text-neutral-content p-3 rounded-lg m-2 font-mono text-xs whitespace-pre-wrap break-all",
                    div { class: "text-gray-400 border-b border-gray-600 pb-1 mb-1 flex justify-between items-center",
                        span { "Console Output" }
                        span { class: "text-gray-500", "{exit_info()}" }
                    }
                    {output()}
                }
            }
            if !error_msg().is_empty() {
                div { class: "px-3 pb-2 text-xs text-error",
                    {error_msg()}
                }
            }
        }
    }
}

/// 把 ExecStatus 映射成中文标签。
fn status_label(status: &ExecStatus) -> String {
    match status {
        ExecStatus::Queued => "排队中".to_string(),
        ExecStatus::Running => "运行中".to_string(),
        ExecStatus::Success => "成功".to_string(),
        ExecStatus::Timeout => "超时".to_string(),
        ExecStatus::OomKilled => "内存超限".to_string(),
        ExecStatus::Error => "运行错误".to_string(),
        ExecStatus::Failed => "系统失败".to_string(),
        ExecStatus::RateLimited => "请求过频".to_string(),
    }
}

/// 生成一个伪唯一后缀（基于时间戳 + 计数器），用于容器 id。
/// 非安全用途，仅避免同页多实例 id 冲突。
fn now_pseudo_unique() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}", crate::utils::time::now_millis(), n)
}

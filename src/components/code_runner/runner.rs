//! CodeRunner 组件实现：源码 + 运行按钮 + 轮询 + 输出区。
//!
//! 编辑器挂载：组件在 WASM 端按自身 `container_id` 调用
//! `codemirror_bridge::get_module().create(...)` 挂载 CodeMirror，`onChange`
//! 回写到内部 `source_signal`，`use_drop` 时销毁实例。范式镜像 SQL 控制台
//!（`src/pages/admin/system.rs`）与 Tiptap 编辑器（`src/pages/admin/write.rs`）。

use dioxus::prelude::*;

use crate::api::code_runner::execute::{get_exec_result, start_exec};
use crate::api::code_runner::{ExecRequest, ExecStatus};
use crate::components::ui::SPINNER_SVG;
use crate::infra::runner_config::ResourceLimits;
use crate::utils::time::sleep_ms;

/// 轮询间隔（毫秒）。
const POLL_INTERVAL_MS: u32 = 500;
/// 轮询最大次数兜底，避免任务卡在 Running 状态时无限轮询。
const MAX_POLLS: u32 = 240; // 500ms * 240 = 120s 上限

/// 代码运行器组件。
///
/// Props：
/// - `source`：初始源码（首次挂载用于初始化编辑器；之后编辑器内容是唯一真源）。
/// - `language`：语言标识（python / node 等）。
/// - `overrides`：可选资源限制覆盖。
/// - `instance_id`：实例在父级片段序列中的索引，用作 CodeMirror 容器 id 后缀。
///   必须是 **SSR/hydration 确定性**的值（如父组件 `for (i, ..)` 的索引 `i`）——
///   Dioxus hydration 不传递 use_hook 状态，任何在 use_hook 内基于运行时状态
///   （时间戳 / 随机 / ScopeId）生成的 id，在 SSR 与 hydration 两端会不一致，
///   导致 CodeMirror `create()` 在 hydration 时找不到 SSR 渲染的容器元素。
///
/// `mut` 信号仅在 WASM 的 spawn 闭包内被 `.set()`，server 构建会触发 unused_mut，
/// 故按项目惯例加 `cfg_attr` 放行（参见 AGENTS.md「mut bindings needed only on WASM」）。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
pub fn CodeRunner(
    source: String,
    language: String,
    overrides: Option<ResourceLimits>,
    instance_id: usize,
) -> Element {
    let mut running = use_signal(|| false);
    let mut stage = use_signal(String::new);
    let mut output = use_signal(String::new);
    let mut exit_info = use_signal(String::new);
    let mut error_msg = use_signal(String::new);

    // 编辑器内容的唯一真源；初始化为 prop 值。
    let mut source_signal = use_signal(|| source.clone());

    // CodeMirror 容器 id：直接由确定性 prop 派生（不进 use_hook）。
    // instance_id 由父组件从纯函数片段解析的索引传入，SSR 与 hydration 同一 content_html
    // → 同一片段序列 → 同一索引 → 同一 id，故 hydration 时 create() 能找到 SSR 渲染的容器。
    let container_id = format!("code-runner-{instance_id}");

    // —— CodeMirror 挂载（仅 WASM）——
    // 范式镜像 src/pages/admin/system.rs 的 SQL 控制台与 src/pages/admin/write.rs 的 Tiptap。
    #[cfg(target_arch = "wasm32")]
    {
        use crate::codemirror_bridge;
        use crate::theme::{use_resolved_theme, ResolvedTheme};
        use wasm_bindgen::closure::Closure;

        let mut editor_handle: Signal<Option<codemirror_bridge::EditorHandle>> =
            use_signal(|| None);

        // 首次挂载：构造 closure + options，create 后存进 editor_handle。
        // 用 resolved() 读取主题作为初始值（同时订阅，但主题切换由下方独立 effect 处理）。
        let mount_language = language.clone();
        let mount_container_id = container_id.clone();
        use_effect(move || {
            if editor_handle.read().is_some() {
                return; // 防重复 init
            }

            // onChange 回写到 source_signal（编辑器内容 = 唯一真源）。
            let on_change = Closure::new({
                let mut sig = source_signal;
                move |v: String| sig.set(v)
            });
            let on_ready = Closure::new(|| {});
            // CodeRunner 不使用 Ctrl+Enter 运行快捷键（它有自己的运行按钮），
            // 但 EditorHandle 签名要求该闭包，传 no-op 满足生命周期。
            let on_run_shortcut = Closure::new(|| {});

            let resolved = use_resolved_theme();
            let theme_name = if resolved() == ResolvedTheme::Dark {
                "dark"
            } else {
                "light"
            };

            let opts = codemirror_bridge::EditorOptions::new();
            opts.set_language(&mount_language);
            opts.set_theme(theme_name);
            opts.set_value(&source_signal.read());
            opts.set_on_change(&on_change);
            opts.set_on_ready(&on_ready);
            opts.set_on_run_shortcut(&on_run_shortcut);

            if let Ok(Some(inst)) =
                codemirror_bridge::get_module().create(&mount_container_id, &opts)
            {
                let handle = codemirror_bridge::EditorHandle::new(
                    inst,
                    on_change,
                    on_ready,
                    on_run_shortcut,
                );
                editor_handle.set(Some(handle));
            }
        });

        // 主题切换（含 System 模式下系统偏好变化）时同步编辑器主题。
        use_effect(move || {
            let r = use_resolved_theme();
            if let Some(h) = editor_handle.read().as_ref() {
                h.instance()
                    .set_theme(if r() == ResolvedTheme::Dark {
                        "dark"
                    } else {
                        "light"
                    });
            }
        });

        // source prop 外部变更（如 admin 页面切换语言重置示例代码）同步到
        // signal + 编辑器。读取 source() 订阅其变化；编辑器用户输入经 on_change
        // 写回 signal，但不会改变 prop（单向），故不会形成回环。
        let sync_source = source.clone();
        use_effect(move || {
            if sync_source != *source_signal.read() {
                source_signal.set(sync_source.clone());
                if let Some(h) = editor_handle.read().as_ref() {
                    h.instance().set_value(&sync_source);
                }
            }
        });

        // 组件卸载时销毁 CodeMirror 实例（EditorHandle::drop → instance.destroy）。
        use_drop(move || {
            editor_handle.set(None);
        });
    }

    // —— run_code：同步段取 signal 当前值，move 进 spawn ——
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

        // 不能跨 await 持有 signal borrow，先 clone 出 owned 值。
        let run_source = source_signal.read().clone();
        let req = ExecRequest {
            language: run_language.clone(),
            source: run_source,
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
        div { class: "rounded-2xl overflow-hidden border border-[var(--color-paper-border)] bg-[var(--color-paper-entry)]",
            // 顶栏：语言标签 + 运行按钮
            div { class: "flex justify-between items-center px-4 py-2.5 border-b border-[var(--color-paper-border)] bg-[var(--color-paper-theme)]",
                div { class: "flex items-center gap-2",
                    span { class: "w-2 h-2 rounded-full bg-[var(--color-paper-accent)]" }
                    span { class: "font-mono text-sm font-semibold text-[var(--color-paper-primary)]", "{language}" }
                }
                button {
                    class: "inline-flex items-center gap-1.5 px-4 py-1.5 text-sm font-medium rounded-full text-[var(--color-paper-theme)] bg-[var(--color-paper-accent)] hover:brightness-110 active:scale-[0.98] transition disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer",
                    disabled: running(),
                    onclick: run_code,
                    if running() {
                        span { class: "inline-block w-3.5 h-3.5 text-[var(--color-paper-theme)]",
                            dangerous_inner_html: SPINNER_SVG,
                        }
                        "{stage()}"
                    } else {
                        "运行"
                    }
                }
            }
            // CodeMirror 容器：组件自身在 WASM 端按此 id 挂载编辑器。
            // 服务端渲染时仅展示空容器，避免在 SSR 阶段拉起 JS 编辑器。
            div {
                id: "{container_id}",
                class: "code-runner-editor font-mono text-sm",
                style: "min-height: 160px; display: flex; flex-direction: column",
            }
            // 输出区
            if !output().is_empty() {
                div { class: "border-t border-[var(--color-paper-border)]",
                    div { class: "flex justify-between items-center px-4 py-2 text-xs text-[var(--color-paper-tertiary)] border-b border-[var(--color-paper-border)] bg-[var(--color-paper-code-block)]",
                        span { class: "font-medium uppercase tracking-wide", "输出" }
                        span { "{exit_info()}" }
                    }
                    pre { class: "px-4 py-3 m-0 text-xs font-mono text-[var(--color-paper-secondary)] bg-[var(--color-paper-code-block)] overflow-x-auto whitespace-pre-wrap break-words",
                        {output()}
                    }
                }
            }
            // 错误提示
            if !error_msg().is_empty() {
                div { class: "px-4 py-2.5 border-t border-[var(--color-paper-border)] text-xs text-red-500 dark:text-red-400 bg-red-50 dark:bg-red-900/10",
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

//! 后台系统管理页面（数据库 + 服务器状态 + SQL 控制台 + 导出 + 备份）。
//!
//! 用顶部 tab 切换 5 个功能，tab 状态用 `use_signal`（不深链 / 不走分页路由）。
//! 各 tab 的实际内容由后续 task 填充，本文件先搭好骨架与 tab 切换。

use dioxus::prelude::*;

/// 系统管理的 5 个功能 tab。
#[derive(Clone, Copy, PartialEq, Debug)]
enum SystemTab {
    /// 数据库运行状态（表/连接/死元组/迁移版本）。
    DbStatus,
    /// 服务器状态（应用内 + 主机层 CPU/内存/磁盘）。
    ServerStatus,
    /// SQL 控制台（全读写 + 护栏）。
    SqlConsole,
    /// 数据导出（SQL/CSV 流式下载）。
    Export,
    /// 备份恢复（pg_dump + 任务进度）。
    Backup,
}

/// 系统管理入口组件。
#[component]
pub fn System() -> Element {
    // tab 状态：默认进第一个 tab（数据库状态）。用 signal 而非 URL query——
    // tab 切换无需深链/书签，避免新增路由变体。
    let mut active_tab = use_signal(|| SystemTab::DbStatus);

    let tabs = [
        ("数据库状态", SystemTab::DbStatus),
        ("服务器状态", SystemTab::ServerStatus),
        ("SQL 控制台", SystemTab::SqlConsole),
        ("数据导出", SystemTab::Export),
        ("备份恢复", SystemTab::Backup),
    ];

    rsx! {
        div { class: "space-y-6",
            h1 { class: "text-2xl font-bold text-paper-primary", "系统管理" }

            // 顶部 tab 切换栏
            div { class: "flex flex-wrap gap-1 border-b border-paper-border",
                for (label, tab) in tabs {
                    button {
                        key: "{tab:?}",
                        class: "px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors",
                        class: if active_tab() == tab {
                            "border-paper-accent text-paper-accent"
                        } else {
                            "border-transparent text-paper-secondary hover:text-paper-primary"
                        },
                        onclick: move |_| active_tab.set(tab),
                        {label}
                    }
                }
            }

            // tab 内容
            div {
                match active_tab() {
                    SystemTab::DbStatus => rsx! { DbStatusTab {} },
                    SystemTab::ServerStatus => rsx! { div { class: "text-paper-secondary py-8", "服务器状态（待实现）" } },
                    SystemTab::SqlConsole => rsx! { div { class: "text-paper-secondary py-8", "SQL 控制台（待实现）" } },
                    SystemTab::Export => rsx! { div { class: "text-paper-secondary py-8", "数据导出（待实现）" } },
                    SystemTab::Backup => rsx! { div { class: "text-paper-secondary py-8", "备份恢复（待实现）" } },
                }
            }
        }
    }
}

/// 字节数 → 人类可读（如 1.2 MB）。
fn format_bytes(bytes: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size.abs() >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit])
    }
}

/// WASM 端异步 sleep（用 web_sys setTimeout 包成 JsFuture，避免引入新依赖）。
/// 非 wasm32 平台立即返回（自动刷新只在 WASM 前端用）。
#[cfg(target_arch = "wasm32")]
async fn wasm_sleep(ms: u32) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .expect("no window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve.unchecked_into(),
                ms,
            )
            .expect("set_timeout failed");
    });
    let _ = JsFuture::from(promise).await;
}

/// 数据库状态 tab：概览卡片 + 表清单 + 索引 Top + 活跃连接。
/// 手动刷新按钮 + 自动刷新开关（1s/2s/5s/30s/手动，默认手动）。
#[allow(non_snake_case)]
fn DbStatusTab() -> Element {
    use crate::api::database::status::DbStatus;
    // get_db_status 只在 WASM 前端调用，server 构建时该 server function 的客户端桩不需要导入。
    #[cfg(target_arch = "wasm32")]
    use crate::api::database::status::get_db_status;
    use crate::components::ui::{ADMIN_CARD_CLASS, ADMIN_TABLE_CLASS};

    // Signal 是 Copy，可在多个 spawn/effect 中捕获同一副本；set 走内部可变（&self）。
    let status = use_signal(|| Option::<DbStatus>::None);
    let mut loading = use_signal(|| true);
    let error = use_signal(|| Option::<String>::None);
    // 自动刷新间隔（秒）；None = 手动。DB 查询有成本，最低 1s。
    let mut refresh_interval: Signal<Option<u32>> = use_signal(|| None);

    // 数据加载：WASM 前端 spawn 请求，SSR 直接结束加载。
    // 因 Signal 是 Copy，每次 spawn 各自捕获副本即可，无需共享闭包。
    let mut load_once = move || {
        loading.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            spawn(async move {
                match get_db_status().await {
                    Ok(s) => {
                        status.set(Some(s));
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            loading.set(false);
        }
    };

    // 首次加载
    use_effect(move || {
        load_once();
    });

    // 自动刷新：interval 变化时 use_future 重新执行，内部周期 sleep 后重新 load。
    // wasm32 才有意义（SSR 不轮询）；sleep 用 web_sys 的 setTimeout 包成 JsFuture，
    // 避免引入新依赖。interval 在 use_future 依赖里读取，变化即重建 future。
    use_future(move || {
        // 在依赖闭包里读 interval，使 use_future 在它变化时重新执行。
        let interval_secs = refresh_interval();
        let status_f = status;
        let loading_f = loading;
        let error_f = error;
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let secs = interval_secs.unwrap_or(0);
                if secs == 0 {
                    return;
                }
                loop {
                    wasm_sleep(secs * 1000).await;
                    loading_f.set(true);
                    spawn(async move {
                        match get_db_status().await {
                            Ok(s) => {
                                status_f.set(Some(s));
                                error_f.set(None);
                            }
                            Err(e) => error_f.set(Some(e.to_string())),
                        }
                        loading_f.set(false);
                    });
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (interval_secs, status_f, loading_f, error_f);
            }
        }
    });

    // Option<DbStatus> 非 Copy，读出来克隆一份供 rsx 消费。
    let current = status.read().clone();

    rsx! {
        div { class: "space-y-6",
            // 工具栏：刷新按钮 + 自动刷新开关
            div { class: "flex items-center justify-between",
                button {
                    class: "px-3 py-1.5 text-sm bg-paper-accent text-paper-theme rounded hover:brightness-110 transition disabled:opacity-50",
                    disabled: loading(),
                    onclick: move |_| {
                        loading.set(true);
                        #[cfg(target_arch = "wasm32")]
                        {
                            spawn(async move {
                                match get_db_status().await {
                                    Ok(s) => {
                                        status.set(Some(s));
                                        error.set(None);
                                    }
                                    Err(e) => error.set(Some(e.to_string())),
                                }
                                loading.set(false);
                            });
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            loading.set(false);
                        }
                    },
                    if loading() { "加载中..." } else { "刷新" }
                }
                div { class: "flex items-center gap-2",
                    span { class: "text-sm text-paper-secondary", "自动刷新" }
                    select {
                        class: "text-sm border border-paper-border rounded px-2 py-1 bg-paper-theme text-paper-primary",
                        value: "{refresh_interval().map(|s| s.to_string()).unwrap_or_default()}",
                        onchange: move |e| {
                            let v = e.value();
                            refresh_interval.set(match v.as_str() {
                                "1" => Some(1),
                                "2" => Some(2),
                                "5" => Some(5),
                                "30" => Some(30),
                                _ => None,
                            });
                        },
                        option { value: "", "手动" }
                        option { value: "1", "1s" }
                        option { value: "2", "2s" }
                        option { value: "5", "5s" }
                        option { value: "30", "30s" }
                    }
                }
            }

            if let Some(err) = error.read().clone() {
                div { class: "bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 text-sm text-red-700 dark:text-red-300",
                    "加载失败：{err}"
                }
            } else if let Some(s) = current {
                // 概览卡片
                div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "数据库总大小" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary", "{format_bytes(s.db_size_bytes)}" }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "连接数" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary", "{s.total_connections} / {s.max_connections}" }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "表数量" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary", "{s.tables.len()}" }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "迁移版本" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary truncate",
                            {s.migration_version.clone().unwrap_or_else(|| "—".to_string())}
                        }
                    }
                }

                // 表清单
                div { class: "{ADMIN_TABLE_CLASS}",
                    div { class: "px-4 py-3 border-b border-paper-border text-sm font-medium text-paper-primary",
                        "表清单（~行数为估算）"
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "border-b border-paper-border text-left text-paper-secondary",
                                    th { class: "px-4 py-2 font-medium", "表名" }
                                    th { class: "px-4 py-2 font-medium text-right", "~行数" }
                                    th { class: "px-4 py-2 font-medium text-right", "表大小" }
                                    th { class: "px-4 py-2 font-medium text-right", "索引大小" }
                                    th { class: "px-4 py-2 font-medium text-right", "总大小" }
                                    th { class: "px-4 py-2 font-medium text-right", "死元组" }
                                }
                            }
                            tbody {
                                for t in s.tables.iter() {
                                    tr { class: "border-b border-paper-border last:border-0 hover:bg-paper-entry transition-colors",
                                        td { class: "px-4 py-2 font-mono text-paper-primary", "{t.name}" }
                                        td { class: "px-4 py-2 text-right text-paper-secondary", "{t.row_estimate}" }
                                        td { class: "px-4 py-2 text-right text-paper-secondary", "{format_bytes(t.table_size_bytes)}" }
                                        td { class: "px-4 py-2 text-right text-paper-secondary", "{format_bytes(t.index_size_bytes)}" }
                                        td { class: "px-4 py-2 text-right text-paper-primary font-medium", "{format_bytes(t.total_size_bytes)}" }
                                        td { class: "px-4 py-2 text-right text-paper-secondary", "{t.dead_tuples}" }
                                    }
                                }
                            }
                        }
                    }
                }

                // 索引占用 Top
                if !s.top_indexes.is_empty() {
                    div { class: "{ADMIN_TABLE_CLASS}",
                        div { class: "px-4 py-3 border-b border-paper-border text-sm font-medium text-paper-primary",
                            "索引占用 Top 10"
                        }
                        div { class: "overflow-x-auto",
                            table { class: "w-full text-sm",
                                thead {
                                    tr { class: "border-b border-paper-border text-left text-paper-secondary",
                                        th { class: "px-4 py-2 font-medium", "索引名" }
                                        th { class: "px-4 py-2 font-medium", "所属表" }
                                        th { class: "px-4 py-2 font-medium text-right", "大小" }
                                    }
                                }
                                tbody {
                                    for i in s.top_indexes.iter() {
                                        tr { class: "border-b border-paper-border last:border-0 hover:bg-paper-entry transition-colors",
                                            td { class: "px-4 py-2 font-mono text-paper-primary", "{i.name}" }
                                            td { class: "px-4 py-2 font-mono text-paper-secondary", "{i.table_name}" }
                                            td { class: "px-4 py-2 text-right text-paper-secondary", "{format_bytes(i.size_bytes)}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 活跃连接
                div { class: "{ADMIN_TABLE_CLASS}",
                    div { class: "px-4 py-3 border-b border-paper-border text-sm font-medium text-paper-primary",
                        "活跃连接（{s.active_connections.len()}）"
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "border-b border-paper-border text-left text-paper-secondary",
                                    th { class: "px-4 py-2 font-medium", "PID" }
                                    th { class: "px-4 py-2 font-medium", "用户" }
                                    th { class: "px-4 py-2 font-medium", "状态" }
                                    th { class: "px-4 py-2 font-medium text-right", "时长(秒)" }
                                    th { class: "px-4 py-2 font-medium", "查询" }
                                }
                            }
                            tbody {
                                for c in s.active_connections.iter() {
                                    tr { class: "border-b border-paper-border last:border-0 hover:bg-paper-entry transition-colors",
                                        td { class: "px-4 py-2 text-paper-secondary", "{c.pid}" }
                                        td { class: "px-4 py-2 text-paper-secondary", "{c.user}" }
                                        td { class: "px-4 py-2 text-paper-secondary",
                                            {c.state.clone().unwrap_or_else(|| "—".to_string())}
                                        }
                                        td { class: "px-4 py-2 text-right text-paper-secondary",
                                            {c.query_duration_secs.map(|d| format!("{:.1}", d)).unwrap_or_else(|| "—".to_string())}
                                        }
                                        td { class: "px-4 py-2 font-mono text-xs text-paper-secondary max-w-md truncate",
                                            {c.query.clone().unwrap_or_else(|| "—".to_string())}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if loading() {
                div { class: "text-paper-secondary py-8", "加载中..." }
            } else {
                div { class: "text-paper-secondary py-8", "暂无数据" }
            }
        }
    }
}

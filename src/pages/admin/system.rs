//! 后台系统管理页面（数据库 + 服务器状态 + SQL 控制台 + 导出 + 备份）。
//!
//! 用顶部 tab 切换 5 个功能，tab 状态用 `use_signal`（不深链 / 不走分页路由）。
//! 各 tab 的实际内容由后续 task 填充，本文件先搭好骨架与 tab 切换。

use dioxus::prelude::*;

use crate::components::ui::FilterTabs;

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

impl SystemTab {
    /// 变体 → 稳定字符串 key(用于与基于 String 的 `FilterTabs` 组件桥接)。
    /// 改这些 key 会破坏潜在的持久化/调试场景,见 `from_str` 的反向映射。
    fn as_str(&self) -> &'static str {
        match self {
            SystemTab::DbStatus => "db_status",
            SystemTab::ServerStatus => "server_status",
            SystemTab::SqlConsole => "sql_console",
            SystemTab::Export => "export",
            SystemTab::Backup => "backup",
        }
    }

    /// 字符串 key → 变体。未知/空串返回 Err(调用方 fallback 到默认 tab)。
    /// 与 `as_str` 严格对应;大小写敏感。
    fn from_str(s: &str) -> Result<SystemTab, &'static str> {
        match s {
            "db_status" => Ok(SystemTab::DbStatus),
            "server_status" => Ok(SystemTab::ServerStatus),
            "sql_console" => Ok(SystemTab::SqlConsole),
            "export" => Ok(SystemTab::Export),
            "backup" => Ok(SystemTab::Backup),
            _ => Err("unknown tab key"),
        }
    }
}

/// 系统管理入口组件。
#[component]
pub fn System() -> Element {
    // tab 状态：默认进第一个 tab（数据库状态）。用 signal 而非 URL query——
    // tab 切换无需深链/书签，避免新增路由变体。
    let mut active_tab = use_signal(|| SystemTab::DbStatus);

    rsx! {
        div { class: "w-full max-w-7xl mx-auto space-y-6",
            // 页面标题
            div { class: "flex flex-col md:flex-row md:items-end justify-between gap-6 pb-6 border-b border-[var(--color-paper-border)] mb-6",
                div {
                    h1 { class: "text-4xl font-extrabold tracking-tight text-[var(--color-paper-primary)]", "系统面板" }
                    p { class: "text-base text-[var(--color-paper-secondary)] mt-2", "数据库与服务器诊断" }
                }
            }

            // 顶部 tab 切换栏:复用公共 FilterTabs 组件(String API,经 as_str/from_str 桥接枚举)。
            // 视觉与评论页一致:平滑滑动指示条 + 选中文字 text-paper-primary。
            FilterTabs {
                items: vec![
                    ("db_status", "数据库状态"),
                    ("server_status", "服务器状态"),
                    ("sql_console", "SQL 控制台"),
                    ("export", "数据导出"),
                    ("backup", "备份恢复"),
                ],
                active_value: active_tab().as_str().to_string(),
                on_change: move |v: String| {
                    // 未知 key fallback 到默认 tab,保证状态始终有效。
                    active_tab.set(SystemTab::from_str(&v).unwrap_or(SystemTab::DbStatus));
                },
            }

            // tab 内容
            div {
                match active_tab() {
                    SystemTab::DbStatus => rsx! { DbStatusTab {} },
                    SystemTab::ServerStatus => rsx! { ServerStatusTab {} },
                    SystemTab::SqlConsole => rsx! { SqlConsoleTab {} },
                    SystemTab::Export => rsx! { ExportTab {} },
                    SystemTab::Backup => rsx! { BackupTab {} },
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
                ms.try_into().unwrap(),
            )
            .expect("set_timeout failed");
    });
    let _ = JsFuture::from(promise).await;
}

/// 数据库状态 tab：概览卡片 + 表清单 + 索引 Top + 活跃连接。
/// 手动刷新按钮 + 自动刷新开关（1s/2s/5s/30s/手动，默认手动）。
#[allow(non_snake_case)]
// status/error/loading 在 spawn/onclick 闭包里 .set()，仅 WASM 前端真正用到；
// server 构建里这些 set 调用都在被剥离的 #[cfg(wasm32)] 块内，故 allow unused_mut。
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
fn DbStatusTab() -> Element {
    use crate::api::database::status::DbStatus;
    // get_db_status 只在 WASM 前端调用，server 构建时该 server function 的客户端桩不需要导入。
    #[cfg(target_arch = "wasm32")]
    use crate::api::database::status::get_db_status;
    use crate::components::ui::{ADMIN_CARD_CLASS, ADMIN_TABLE_CLASS};

    // Signal 是 Copy，可在多个 spawn/effect 中捕获同一副本；set 走内部可变（&self）。
    let mut status = use_signal(|| Option::<DbStatus>::None);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);
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

    // 自动刷新：使用官方推荐模式——一个永不重建的长生命周期 loop，在每次循环
    // 内部读取 refresh_interval 的当前值，自然响应间隔切换。
    // 旧做法在闭包同步体内读 status/loading/error signal，导致这些 signal 每次
    // .set() 后都触发 use_future 重建，产生多个并发 loop（请求爆炸）。
    use_future(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            loop {
                // 每次循环读最新 interval（signal 的 Copy 语义，直接调用即可）。
                let secs = refresh_interval().unwrap_or(0);
                if secs == 0 {
                    // 手动模式：短暂 yield，让事件循环呼吸，避免忙等；
                    // 用户切换到自动模式后最多等 200ms 即响应。
                    wasm_sleep(200).await;
                    continue;
                }
                wasm_sleep(secs * 1000).await;
                // 二次检查：sleep 期间用户可能切回手动。
                if refresh_interval().is_none() {
                    continue;
                }
                loading.set(true);
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
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (status, loading, error, refresh_interval);
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

                // 表清单（小表显示真实行数；大表回退估算，行数前标 ~）
                div { class: "{ADMIN_TABLE_CLASS}",
                    div { class: "px-4 py-3 border-b border-paper-border text-sm font-medium text-paper-primary",
                        "表清单（行数：小表为真实值，大表标 ~ 为估算）"
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "border-b border-paper-border text-left text-paper-secondary",
                                    th { class: "px-4 py-2 font-medium", "表名" }
                                    th { class: "px-4 py-2 font-medium text-right", "行数" }
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
                                        td { class: "px-4 py-2 text-right text-paper-secondary",
                                            if t.row_count_estimated { "~{t.row_count}" } else { "{t.row_count}" }
                                        }
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

/// 秒数 → 人类可读运行时间（如 1d 2h 3m）。
fn format_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m")
    } else {
        format!("{secs}s")
    }
}

/// 服务器状态 tab：应用内指标（连接池/会话/缓存命中率）+ 主机层（CPU/内存/磁盘）。
/// 手动刷新 + 自动刷新开关（500ms/1s/2s/5s/手动，默认手动）。
/// 主机层数据由后台 500ms 采样，前端轮询只读快照零成本，故可高频。
#[allow(non_snake_case)]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
fn ServerStatusTab() -> Element {
    use crate::api::database::system_status::ServerStatus;
    #[cfg(target_arch = "wasm32")]
    use crate::api::database::system_status::get_server_status;
    use crate::components::ui::{ADMIN_CARD_CLASS, ADMIN_TABLE_CLASS};

    let mut status = use_signal(|| Option::<ServerStatus>::None);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);
    // 自动刷新间隔（毫秒）；None = 手动。主机层后台采样，前端可高频轮询。
    let mut refresh_ms: Signal<Option<u32>> = use_signal(|| None);

    let mut load_once = move || {
        loading.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            spawn(async move {
                match get_server_status().await {
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

    use_effect(move || {
        load_once();
    });

    // 自动刷新：同 DbStatusTab，采用官方推荐的单一长生命周期 loop 模式。
    // 闭包体内不读任何 signal（避免隐式依赖追踪导致重建），loop 内部实时读取。
    use_future(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            loop {
                let ms = refresh_ms().unwrap_or(0);
                if ms == 0 {
                    wasm_sleep(200).await;
                    continue;
                }
                wasm_sleep(ms).await;
                if refresh_ms().is_none() {
                    continue;
                }
                loading.set(true);
                spawn(async move {
                    match get_server_status().await {
                        Ok(s) => {
                            status.set(Some(s));
                            error.set(None);
                        }
                        Err(e) => error.set(Some(e.to_string())),
                    }
                    loading.set(false);
                });
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (status, loading, error, refresh_ms);
        }
    });

    let current = status.read().clone();
    // rsx 不支持格式说明符（{:.1}），也不允许在 for 循环体内 let，故预格式化所有展示值。
    let cpu_pct = current
        .as_ref()
        .map(|s| format!("{:.1}%", s.host.cpu_usage))
        .unwrap_or_default();
    let load_1 = current
        .as_ref()
        .map(|s| format!("{:.2}", s.host.load_avg_1))
        .unwrap_or_default();
    // 缓存表预格式化：把每行需要展示的值都算好字符串，避免在 rsx 里做格式化。
    let cache_rows: Vec<(String, u64, u64, u64, String)> = current
        .as_ref()
        .map(|s| {
            s.caches
                .iter()
                .map(|c| {
                    (
                        c.name.clone(),
                        c.entry_count,
                        c.hits,
                        c.misses,
                        format!("{:.1}%", c.hit_rate * 100.0),
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    rsx! {
        div { class: "space-y-6",
        div { class: "flex items-center justify-between",
                button {
                    class: "px-3 py-1.5 text-sm bg-paper-accent text-paper-theme rounded hover:brightness-110 transition disabled:opacity-50",
                    disabled: loading(),
                    onclick: move |_| {
                        loading.set(true);
                        #[cfg(target_arch = "wasm32")]
                        {
                            spawn(async move {
                                match get_server_status().await {
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
                        value: "{refresh_ms().map(|s| s.to_string()).unwrap_or_default()}",
                        onchange: move |e| {
                            let v = e.value();
                            refresh_ms.set(match v.as_str() {
                                "500" => Some(500),
                                "1000" => Some(1000),
                                "2000" => Some(2000),
                                "5000" => Some(5000),
                                _ => None,
                            });
                        },
                        option { value: "", "手动" }
                        option { value: "500", "500ms" }
                        option { value: "1000", "1s" }
                        option { value: "2000", "2s" }
                        option { value: "5000", "5s" }
                    }
                }
            }

            if let Some(err) = error.read().clone() {
                div { class: "bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 text-sm text-red-700 dark:text-red-300",
                    "加载失败：{err}"
                }
            } else if let Some(s) = current {
                // 应用内指标卡片
                div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "运行时间" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary", "{format_uptime(s.uptime_secs)}" }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "DB 连接池" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary", "{s.pool_size} / {s.pool_max_size}" }
                        p { class: "text-xs text-paper-secondary", "空闲 {s.pool_available} · 等待 {s.pool_waiting}" }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "活跃会话" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary", "{s.active_sessions}" }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "CPU" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary", "{cpu_pct}" }
                    }
                }

                // 主机层指标卡片
                div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "内存" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary", "{format_bytes(s.host.used_memory as i64)} / {format_bytes(s.host.total_memory as i64)}" }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "磁盘" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary", "{format_bytes((s.host.disk_total - s.host.disk_available) as i64)} / {format_bytes(s.host.disk_total as i64)}" }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "Load (1m)" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary", "{load_1}" }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "系统" }
                        p { class: "mt-1 text-sm font-medium text-paper-primary truncate", "{s.host.os_name}" }
                    }
                }

                // 缓存命中率表
                div { class: "{ADMIN_TABLE_CLASS}",
                    div { class: "px-4 py-3 border-b border-paper-border text-sm font-medium text-paper-primary",
                        "缓存命中率"
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "border-b border-paper-border text-left text-paper-secondary",
                                    th { class: "px-4 py-2 font-medium", "缓存" }
                                    th { class: "px-4 py-2 font-medium text-right", "条目" }
                                    th { class: "px-4 py-2 font-medium text-right", "命中" }
                                    th { class: "px-4 py-2 font-medium text-right", "未命中" }
                                    th { class: "px-4 py-2 font-medium text-right", "命中率" }
                                }
                            }
                            tbody {
                                for (name, entry_count, hits, misses, rate_pct) in cache_rows.iter() {
                                    tr { class: "border-b border-paper-border last:border-0 hover:bg-paper-entry transition-colors",
                                        td { class: "px-4 py-2 text-paper-primary", "{name}" }
                                        td { class: "px-4 py-2 text-right text-paper-secondary", "{entry_count}" }
                                        td { class: "px-4 py-2 text-right text-paper-secondary", "{hits}" }
                                        td { class: "px-4 py-2 text-right text-paper-secondary", "{misses}" }
                                        td { class: "px-4 py-2 text-right text-paper-primary font-medium", "{rate_pct}" }
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

/// SQL 控制台 tab：CodeMirror 编辑器（SQL 高亮/补全/Vim）+ 4 道护栏 + 结果表 + EXPLAIN。
///
/// 护栏 4（前端二次确认）：提交写操作前弹窗确认。
#[allow(non_snake_case)]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
fn SqlConsoleTab() -> Element {
    use crate::api::database::sql_console::SqlResult;
    #[cfg(target_arch = "wasm32")]
    use crate::api::database::sql_console::{execute_sql, ExecuteSqlOpts};
    #[cfg(target_arch = "wasm32")]
    use crate::api::database::schema::get_db_schema;
    use crate::components::ui::ADMIN_TABLE_CLASS;
    #[cfg(target_arch = "wasm32")]
    use crate::codemirror_bridge;
    // use_resolved_theme 两种构建都用：resolved() 在 wasm 块内消费，非 wasm 仅引用避免警告。
    use crate::theme::use_resolved_theme;
    #[cfg(target_arch = "wasm32")]
    use crate::theme::ResolvedTheme;

    let resolved = use_resolved_theme();
    let sql_text = use_signal(String::new);
    let mut result = use_signal(|| Option::<SqlResult>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut running = use_signal(|| false);
    // 选项 toggles
    let mut with_explain = use_signal(|| false);
    let mut allow_multi = use_signal(|| false);
    let mut confirm_dangerous = use_signal(|| false);
    // resolved/sql_text 仅在 wasm32 块内使用；server 构建时显式引用避免 unused 警告。
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&resolved, &sql_text);

    // CodeMirror 实例句柄（仅 WASM）。
    #[cfg(target_arch = "wasm32")]
    let mut editor_handle: Signal<Option<codemirror_bridge::EditorHandle>> = use_signal(|| None);

    // 初始化 CodeMirror + 拉取 schema 注入补全。仅 WASM。
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::closure::Closure;
        use_effect(move || {
            if editor_handle.read().is_some() {
                return;
            }
            let mut text = sql_text;
            let on_change = Closure::new(move |v: String| {
                text.set(v);
            });
            let on_ready = Closure::new(|| {});

            let theme_name = if resolved() == ResolvedTheme::Dark { "dark" } else { "light" };
            let opts = codemirror_bridge::EditorOptions::new();
            opts.set_language("sql");
            opts.set_theme(theme_name);
            opts.set_vim(true);
            opts.set_on_change(&on_change);
            opts.set_on_ready(&on_ready);

            if let Ok(Some(inst)) = codemirror_bridge::get_module().create("sql-editor", &opts) {
                let handle = codemirror_bridge::EditorHandle::new(inst, on_change, on_ready);
                editor_handle.set(Some(handle));
            }

            // 异步拉取 schema 注入补全
            spawn(async move {
                if let Ok(schema) = get_db_schema().await {
                    // SqlSchema 是 serde 类型：先用 serde_wasm_bindgen 转 JsValue，
                    // 再传给 CodeMirror（extern set_schema 只接受 &JsValue）。
                    if let Ok(js) = serde_wasm_bindgen::to_value(&schema) {
                        if let Some(h) = editor_handle.read().as_ref() {
                            h.instance().set_schema(&js);
                        }
                    }
                }
            });
        });

        // 主题切换（含 System 模式下系统偏好变化）时同步编辑器主题。
        // resolved 是 theme + system_dark 的派生 memo，任一变化都自动触发此 effect。
        use_effect(move || {
            let r = resolved();
            if let Some(h) = editor_handle.read().as_ref() {
                h.instance().set_theme(if r == ResolvedTheme::Dark { "dark" } else { "light" });
            }
        });
    }

    // 执行 SQL
    let mut run_sql = move || {
        running.set(true);
        error.set(None);
        #[cfg(target_arch = "wasm32")]
        {
            let sql = sql_text.read().clone();
            let opts = ExecuteSqlOpts {
                allow_multi: allow_multi(),
                confirm_dangerous: confirm_dangerous(),
                with_explain: with_explain(),
            };
            // 护栏 4：写操作前端二次确认（简单判断：含 UPDATE/DELETE/INSERT/ALTER/DROP/TRUNCATE/CREATE 关键词）
            let lower = sql.to_lowercase();
            let looks_write = ["update ", "delete ", "insert ", "alter ", "drop ", "truncate ", "create "]
                .iter()
                .any(|k| lower.contains(k));
            if looks_write && !confirm_dangerous() {
                // 简单提示；真正的高危放行靠 confirm_dangerous 开关
                let confirmed = web_sys::window().and_then(|w| {
                    w.confirm_with_message(
                        "这是写操作（修改数据/结构），确认执行？\n\n高危操作（DROP/TRUNCATE/ALTER）还需勾选「我了解后果」。",
                    )
                    .ok()
                });
                if confirmed != Some(true) {
                    running.set(false);
                    return;
                }
            }
            spawn(async move {
                match execute_sql(sql, opts).await {
                    Ok(r) => result.set(Some(r)),
                    Err(e) => error.set(Some(e.to_string())),
                }
                running.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            running.set(false);
        }
    };

    let current_result = result.read().clone();
    let current_error = error.read().clone();
    let elapsed = current_result.as_ref().map(|r| r.elapsed_ms).unwrap_or(0);
    let affected = current_result.as_ref().map(|r| r.affected_rows).unwrap_or(0);
    let stmt_type = current_result
        .as_ref()
        .map(|r| r.statement_type.clone())
        .unwrap_or_default();
    let truncated = current_result.as_ref().map(|r| r.truncated).unwrap_or(false);
    // 结果行预格式化（避免在 rsx for 循环体内格式化）
    let result_rows: Vec<Vec<String>> = current_result
        .as_ref()
        .map(|r| {
            r.rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| match cell {
                            serde_json::Value::Null => "NULL".to_string(),
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default();
    let result_columns: Vec<String> = current_result
        .as_ref()
        .map(|r| r.columns.clone())
        .unwrap_or_default();
    let explain = current_result.as_ref().and_then(|r| r.explain.clone());

    rsx! {
        div { class: "space-y-4",
            // 编辑器容器
            div {
                class: "border border-paper-border rounded-lg overflow-hidden bg-paper-entry",
                id: "sql-editor",
                style: "min-height: 200px"
            }

            // 选项 + 执行按钮
            div { class: "flex flex-wrap items-center gap-3",
                button {
                    class: "px-4 py-1.5 text-sm bg-paper-accent text-paper-theme rounded hover:brightness-110 transition disabled:opacity-50",
                    disabled: running(),
                    onclick: move |_| run_sql(),
                    if running() { "执行中..." } else { "执行 (Ctrl+Enter)" }
                }
                label { class: "flex items-center gap-1 text-sm text-paper-secondary",
                    input {
                        r#type: "checkbox",
                        class: "mr-1",
                        checked: with_explain(),
                        onchange: move |e| with_explain.set(e.checked()),
                    }
                    "EXPLAIN"
                }
                label { class: "flex items-center gap-1 text-sm text-paper-secondary",
                    input {
                        r#type: "checkbox",
                        class: "mr-1",
                        checked: allow_multi(),
                        onchange: move |e| allow_multi.set(e.checked()),
                    }
                    "允许多语句"
                }
                label { class: "flex items-center gap-1 text-sm text-red-600 dark:text-red-400",
                    input {
                        r#type: "checkbox",
                        class: "mr-1",
                        checked: confirm_dangerous(),
                        onchange: move |e| confirm_dangerous.set(e.checked()),
                    }
                    "我了解后果（放开 DROP/TRUNCATE/ALTER）"
                }
            }

            // 错误
            if let Some(err) = current_error {
                div { class: "bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-3 text-sm text-red-700 dark:text-red-300",
                    "{err}"
                }
            }

            // 结果摘要
            if current_result.is_some() {
                div { class: "flex flex-wrap gap-4 text-sm text-paper-secondary",
                    span { "类型：{stmt_type}" }
                    if affected > 0 {
                        span { "影响行数：{affected}" }
                    }
                    span { "耗时：{elapsed}ms" }
                    if truncated {
                        span { class: "text-amber-600 dark:text-amber-400", "结果超过 500 行，已截断" }
                    }
                }
            }

            // 结果表格
            if !result_rows.is_empty() {
                div { class: "{ADMIN_TABLE_CLASS}",
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "border-b border-paper-border text-left text-paper-secondary",
                                    for col in result_columns.iter() {
                                        th { class: "px-4 py-2 font-medium whitespace-nowrap", "{col}" }
                                    }
                                }
                            }
                            tbody {
                                for row in result_rows.iter() {
                                    tr { class: "border-b border-paper-border last:border-0 hover:bg-paper-entry transition-colors",
                                        for cell in row.iter() {
                                            td { class: "px-4 py-2 font-mono text-xs text-paper-secondary", "{cell}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // EXPLAIN 输出
            if let Some(explain) = explain {
                div { class: "{ADMIN_TABLE_CLASS}",
                    div { class: "px-4 py-2 border-b border-paper-border text-sm font-medium text-paper-primary",
                        "执行计划"
                    }
                    pre { class: "p-4 text-xs font-mono text-paper-secondary overflow-x-auto whitespace-pre", "{explain}" }
                }
            }
        }
    }
}

/// 数据导出 tab：按表/按查询导出 SQL/CSV，走 Axum 流式下载。
#[allow(non_snake_case)]
fn ExportTab() -> Element {
    use crate::components::ui::ADMIN_CARD_CLASS;
    // 导出模式："table" / "query"
    let mut mode = use_signal(|| "table".to_string());
    let mut table_name = use_signal(String::new);
    let mut query = use_signal(String::new);
    let mut format = use_signal(|| "csv".to_string());
    let mut include_columns = use_signal(|| true);

    // 触发下载：构造 GET /api/database/export?... URL 并打开
    let do_export = move || {
        #[cfg(target_arch = "wasm32")]
        {
            let source = if mode().as_str() == "table" {
                format!("table:{}", table_name.read().trim())
            } else {
                format!("query:{}", query.read())
            };
            let url = format!(
                "/api/database/export?source={}&format={}&include_columns={}",
                urlencode(&source),
                format(),
                include_columns(),
            );
            if let Some(window) = web_sys::window() {
                let _ = window.open_with_url(&url);
            }
        }
    };

    rsx! {
        div { class: "space-y-4 max-w-2xl",
            div { class: "{ADMIN_CARD_CLASS} p-4 space-y-4",
                // 模式选择
                div { class: "flex items-center gap-4",
                    label { class: "flex items-center gap-2 text-sm text-paper-primary",
                        input {
                            r#type: "radio",
                            name: "export-mode",
                            checked: mode() == "table",
                            onchange: move |_| mode.set("table".to_string()),
                        }
                        "按表导出"
                    }
                    label { class: "flex items-center gap-2 text-sm text-paper-primary",
                        input {
                            r#type: "radio",
                            name: "export-mode",
                            checked: mode() == "query",
                            onchange: move |_| mode.set("query".to_string()),
                        }
                        "按查询导出"
                    }
                }

                // 表名输入
                if mode().as_str() == "table" {
                    div {
                        label { class: "block text-sm text-paper-secondary mb-1", "表名" }
                        input {
                            r#type: "text",
                            class: "w-full px-3 py-2 text-sm border border-paper-border rounded bg-paper-theme text-paper-primary font-mono",
                            placeholder: "如 posts",
                            value: "{table_name}",
                            oninput: move |e| table_name.set(e.value()),
                        }
                        p { class: "text-xs text-paper-secondary mt-1", "仅支持 public schema 下的用户表，表名需为合法标识符" }
                    }
                } else {
                    // 查询输入
                    div {
                        label { class: "block text-sm text-paper-secondary mb-1", "SELECT 查询（只读）" }
                        textarea {
                            class: "w-full px-3 py-2 text-sm border border-paper-border rounded bg-paper-theme text-paper-primary font-mono",
                            rows: "4",
                            placeholder: "SELECT id, title FROM posts WHERE published = true",
                            value: "{query}",
                            oninput: move |e| query.set(e.value()),
                        }
                    }
                }

                // 格式 + 选项
                div { class: "flex flex-wrap items-center gap-4",
                    div { class: "flex items-center gap-2",
                        span { class: "text-sm text-paper-secondary", "格式" }
                        select {
                            class: "text-sm border border-paper-border rounded px-2 py-1 bg-paper-theme text-paper-primary",
                            value: "{format}",
                            onchange: move |e| format.set(e.value()),
                            option { value: "csv", "CSV" }
                            option { value: "sql", "SQL (INSERT)" }
                        }
                    }
                    label { class: "flex items-center gap-1 text-sm text-paper-secondary",
                        input {
                            r#type: "checkbox",
                            class: "mr-1",
                            checked: include_columns(),
                            onchange: move |e| include_columns.set(e.checked()),
                        }
                        "包含列名（CSV 表头 / INSERT 列清单）"
                    }
                }

                button {
                    class: "px-4 py-1.5 text-sm bg-paper-accent text-paper-theme rounded hover:brightness-110 transition",
                    onclick: move |_| do_export(),
                    "导出并下载"
                }
            }
            p { class: "text-xs text-paper-secondary",
                "导出走流式响应，大表不会占满内存。SQL 格式仅含 INSERT 语句（不含 DDL/schema）。"
            }
        }
    }
}

/// 简易 URL 编码（避免引入新依赖；仅编码导出参数里的特殊字符）。
/// 仅在 WASM 前端的导出按钮里用。
#[cfg(target_arch = "wasm32")]
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// 备份恢复 tab：备份按钮 + 进度轮询 + 备份列表（下载/恢复/删除）。
#[allow(non_snake_case)]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
fn BackupTab() -> Element {
    use crate::api::database::backup::BackupInfo;
    use crate::api::database::tasks::TaskProgress;
    #[cfg(target_arch = "wasm32")]
    use crate::api::database::backup::{create_backup, delete_backup, list_backups, restore_backup};
    #[cfg(target_arch = "wasm32")]
    use crate::api::database::tasks::{get_task_progress, TaskStatus};
    use crate::components::ui::{ADMIN_CARD_CLASS, ADMIN_TABLE_CLASS};

    // backups/active_task_id 仅在闭包内的重绑定副本上 .set()（如 backups_f），
    // 外层绑定本身不改值，故无需 mut。
    let backups = use_signal(Vec::<BackupInfo>::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    // 当前进行中的任务（备份/恢复）id + 进度
    let active_task_id: Signal<Option<String>> = use_signal(|| None);
    let mut active_progress = use_signal(|| Option::<TaskProgress>::None);
    // 待恢复的文件名（确认对话框用）
    let mut pending_restore: Signal<Option<String>> = use_signal(|| None);
    let mut busy = use_signal(|| false);

    // 刷新备份列表
    let mut refresh_list = move || {
        loading.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            let mut backups = backups;
            let mut error = error;
            spawn(async move {
                match list_backups().await {
                    Ok(list) => {
                        backups.set(list);
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

    use_effect(move || {
        refresh_list();
    });

    // 恢复确认：pending_restore 被设置时弹出原生 confirm，确认后发起 restore_backup
    // 并开始进度轮询。用 use_future 响应 pending_restore 变化。
    // _pending_for_confirm 被 clone 进 async move（FnMut 可重入），用 Option::clone 兜底。
    let _pending_for_confirm = pending_restore.read().clone();
    use_future(move || {
        let mut pending_restore = pending_restore;
        let mut busy = busy;
        let mut active_progress = active_progress;
        let mut active_task_id = active_task_id;
        let mut error = error;
        let pending_for_confirm = _pending_for_confirm.clone();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let fname = match pending_for_confirm {
                    Some(f) => f,
                    None => return,
                };
                let confirmed = web_sys::window()
                    .and_then(|w| {
                        w.confirm_with_message(&format!(
                            "恢复将覆盖现有数据，确认恢复 {}？\n\n仅本系统生成的备份可恢复。",
                            fname
                        ))
                        .ok()
                    })
                    == Some(true);
                if confirmed {
                    busy.set(true);
                    active_progress.set(None);
                    match restore_backup(fname, true).await {
                        Ok(id) => active_task_id.set(Some(id)),
                        Err(e) => {
                            error.set(Some(e.to_string()));
                            busy.set(false);
                        }
                    }
                }
                // 无论确认与否都清空 pending
                pending_restore.set(None);
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (pending_restore, busy, active_progress, active_task_id, error, pending_for_confirm);
            }
        }
    });

    // 任务进度轮询：active_task_id 存在时每 1.5s 拉取进度，Done/Failed 后停止 + 刷新列表
    // _task_id_for_poll 被 clone 进 async move（FnMut 可重入）。
    let _task_id_for_poll = active_task_id.read().clone();
    use_future(move || {
        let mut active_task_id = active_task_id;
        let mut active_progress = active_progress;
        let mut backups_f = backups;
        let mut busy_f = busy;
        let task_id_for_poll = _task_id_for_poll.clone();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let tid = match task_id_for_poll {
                    Some(t) => t,
                    None => return,
                };
                loop {
                    wasm_sleep(1500).await;
                    match get_task_progress(tid.clone()).await {
                        Ok(p) => {
                            let done = p.status == TaskStatus::Done || p.status == TaskStatus::Failed;
                            active_progress.set(Some(p));
                            if done {
                                // 刷新列表（备份完成后新文件出现）并清理任务态
                                if let Ok(list) = list_backups().await {
                                    backups_f.set(list);
                                }
                                active_task_id.set(None);
                                busy_f.set(false);
                                break;
                            }
                        }
                        Err(_) => {
                            active_task_id.set(None);
                            busy_f.set(false);
                            break;
                        }
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (active_task_id, active_progress, backups_f, busy_f, task_id_for_poll);
            }
        }
    });

    let current_backups = backups.read().clone();
    let current_error = error.read().clone();
    let current_progress = active_progress.read().clone();
    let is_busy = busy();
    // 预格式化备份行（避免在 rsx for 循环体内 let / 格式化）。
    // 每行：(filename, mode, size_str, dl_url)
    let backup_rows: Vec<(String, String, String, String)> = current_backups
        .iter()
        .map(|b| {
            (
                b.filename.clone(),
                b.mode.clone(),
                format_bytes(b.size_bytes as i64),
                format!("/api/database/backups/{}", urlencode_dl(&b.filename)),
            )
        })
        .collect();

    rsx! {
        div { class: "space-y-4",
            // 操作栏
            div { class: "flex items-center gap-3",
                button {
                    class: "px-4 py-1.5 text-sm bg-paper-accent text-paper-theme rounded hover:brightness-110 transition disabled:opacity-50",
                    disabled: is_busy,
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            busy.set(true);
                            active_progress.set(None);
                            let mut active_task_id = active_task_id;
                            spawn(async move {
                                match create_backup().await {
                                    Ok(id) => active_task_id.set(Some(id)),
                                    Err(e) => { error.set(Some(e.to_string())); busy.set(false); }
                                }
                            });
                        }
                    },
                    if is_busy { "处理中..." } else { "创建备份" }
                }
                button {
                    class: "px-3 py-1.5 text-sm border border-paper-border text-paper-primary rounded hover:bg-paper-entry transition disabled:opacity-50",
                    disabled: loading() || is_busy,
                    onclick: move |_| refresh_list(),
                    "刷新列表"
                }
            }

            // 进度
            if let Some(p) = current_progress {
                div { class: "{ADMIN_CARD_CLASS} p-4",
                    div { class: "flex items-center justify-between mb-2",
                        span { class: "text-sm font-medium text-paper-primary", "{p.stage}" }
                        span { class: "text-sm text-paper-secondary", "{p.percent}%" }
                    }
                    div { class: "w-full bg-paper-entry rounded-full h-2 overflow-hidden",
                        div { class: "bg-paper-accent h-full transition-all", style: "width: {p.percent}%" }
                    }
                    if let Some(detail) = p.detail {
                        p { class: "text-xs text-paper-secondary mt-2", "{detail}" }
                    }
                    if let Some(err) = p.error {
                        p { class: "text-xs text-red-600 dark:text-red-400 mt-2", "错误：{err}" }
                    }
                }
            }

            // 错误
            if let Some(err) = current_error {
                div { class: "bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-3 text-sm text-red-700 dark:text-red-300",
                    "{err}"
                }
            }

            // 备份列表
            if !current_backups.is_empty() {
                div { class: "{ADMIN_TABLE_CLASS}",
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "border-b border-paper-border text-left text-paper-secondary",
                                    th { class: "px-4 py-2 font-medium", "文件名" }
                                    th { class: "px-4 py-2 font-medium", "模式" }
                                    th { class: "px-4 py-2 font-medium text-right", "大小" }
                                    th { class: "px-4 py-2 font-medium text-right", "操作" }
                                }
                            }
                            tbody {
                                for (fname, mode, size_str, dl_url) in backup_rows.iter() {
                                    BackupRow {
                                        key: "{fname}",
                                        filename: fname.clone(),
                                        mode: mode.clone(),
                                        size_str: size_str.clone(),
                                        dl_url: dl_url.clone(),
                                        busy: is_busy,
                                        on_restore: move |f: String| pending_restore.set(Some(f)),
                                        on_delete: move |_fname_del: String| {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let fname_del = _fname_del;
                                                let confirmed = web_sys::window()
                                                    .and_then(|w| {
                                                        w.confirm_with_message(&format!("确认删除 {}？", fname_del))
                                                            .ok()
                                                    })
                                                    == Some(true);
                                                if confirmed {
                                                    let mut backups = backups;
                                                    spawn(async move {
                                                        let _ = delete_backup(fname_del.clone()).await;
                                                        if let Ok(list) = list_backups().await {
                                                            backups.set(list);
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
            } else if !loading() {
                div { class: "text-paper-secondary text-sm py-4", "暂无备份文件" }
            }

            p { class: "text-xs text-paper-secondary",
                "备份优先用 pg_dump（含 schema），不可用时回退纯 SQL（仅数据）。"
                "恢复仅接受本系统生成的备份，且会覆盖现有数据。"
            }
        }
    }
}

/// 下载链接用的 URL 编码（wasm32 才用，server 端 rsx 也引用故需编译）。
fn urlencode_dl(s: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        urlencode(s)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        s.to_string()
    }
}

/// 备份列表单行（抽取为子组件：各自 scope 内 let/clone 不冲突）。
#[derive(Props, Clone, PartialEq)]
struct BackupRowProps {
    filename: String,
    mode: String,
    size_str: String,
    dl_url: String,
    busy: bool,
    on_restore: Callback<String>,
    on_delete: Callback<String>,
}

#[component]
fn BackupRow(props: BackupRowProps) -> Element {
    // Callback 是 Copy，直接复用；filename 需 clone（两个闭包各取一份）。
    let on_restore = props.on_restore;
    let on_delete = props.on_delete;
    let fname_for_restore = props.filename.clone();
    let fname_for_delete = props.filename.clone();
    rsx! {
        tr { class: "border-b border-paper-border last:border-0 hover:bg-paper-entry transition-colors",
            td { class: "px-4 py-2 font-mono text-xs text-paper-primary", "{props.filename}" }
            td { class: "px-4 py-2 text-paper-secondary", "{props.mode}" }
            td { class: "px-4 py-2 text-right text-paper-secondary", "{props.size_str}" }
            td { class: "px-4 py-2 text-right whitespace-nowrap",
                a {
                    class: "text-xs text-paper-accent hover:underline mr-3",
                    href: "{props.dl_url}",
                    download: "",
                    "下载"
                }
                button {
                    class: "text-xs text-amber-600 hover:text-amber-800 dark:text-amber-400 mr-3 disabled:opacity-50",
                    disabled: props.busy,
                    onclick: move |_| on_restore.call(fname_for_restore.clone()),
                    "恢复"
                }
                button {
                    class: "text-xs text-red-600 hover:text-red-800 dark:text-red-400 disabled:opacity-50",
                    disabled: props.busy,
                    onclick: move |_| on_delete.call(fname_for_delete.clone()),
                    "删除"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SystemTab;

    #[test]
    fn as_str_roundtrips_all_variants() {
        // 每个变体经 as_str -> from_str 必须还原为自身。
        for tab in [
            SystemTab::DbStatus,
            SystemTab::ServerStatus,
            SystemTab::SqlConsole,
            SystemTab::Export,
            SystemTab::Backup,
        ] {
            let s = tab.as_str();
            assert_eq!(SystemTab::from_str(s), Ok(tab), "roundtrip failed for {tab:?}");
        }
    }

    #[test]
    fn as_str_returns_stable_keys() {
        // 字符串 key 必须稳定(改 key 会破坏 URL/调试/未来持久化),锁定之。
        assert_eq!(SystemTab::DbStatus.as_str(), "db_status");
        assert_eq!(SystemTab::ServerStatus.as_str(), "server_status");
        assert_eq!(SystemTab::SqlConsole.as_str(), "sql_console");
        assert_eq!(SystemTab::Export.as_str(), "export");
        assert_eq!(SystemTab::Backup.as_str(), "backup");
    }

    #[test]
    fn from_str_rejects_unknown_and_empty() {
        assert!(SystemTab::from_str("nonsense").is_err());
        assert!(SystemTab::from_str("").is_err());
        // 大小写敏感:不接受大写变体,避免歧义。
        assert!(SystemTab::from_str("DbStatus").is_err());
    }
}

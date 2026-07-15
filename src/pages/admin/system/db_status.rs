//! 数据库状态 tab。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::ui::LoadingButton;

use super::format_bytes;

/// 数据库状态 tab：概览卡片 + 表清单 + 索引 Top + 活跃连接。
/// 手动刷新按钮 + 自动刷新开关（1s/2s/5s/30s/手动，默认手动）。
#[allow(non_snake_case)]
// status/error/loading 在 spawn/onclick 闭包里 .set()，仅 WASM 前端真正用到；
// server 构建里这些 set 调用都在被剥离的 #[cfg(wasm32)] 块内，故 allow unused_mut。
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
pub(super) fn DbStatusTab() -> Element {
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
            let t0 = crate::utils::time::now_millis();
            web_sys::console::log_1(&format!("[DEBUG-db] fetch start at {t0}").into());
            spawn(async move {
                match get_db_status().await {
                    Ok(s) => {
                        let t1 = crate::utils::time::now_millis();
                        web_sys::console::log_1(
                            &format!("[DEBUG-db] fetch done at {t1} (Δ={}ms)", t1 - t0).into(),
                        );
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
                    crate::utils::time::sleep_ms(200).await;
                    continue;
                }
                crate::utils::time::sleep_ms(secs * 1000).await;
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
                LoadingButton {
                    label: "刷新".to_string(),
                    loading: loading(),
                    variant: "sm",
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
                }
                div { class: "flex items-center gap-2",
                    span { class: "text-sm text-paper-secondary", "自动刷新" }
                    select {
                        class: "text-sm border border-paper-border rounded px-2 py-1 bg-paper-theme text-paper-primary",
                        value: "{refresh_interval().map(|s| s.to_string()).unwrap_or_default()}",
                        onchange: move |e| {
                            let v = e.value();
                            refresh_interval
                                .set(
                                    match v.as_str() {
                                        "1" => Some(1),
                                        "2" => Some(2),
                                        "5" => Some(5),
                                        "30" => Some(30),
                                        _ => None,
                                    },
                                );
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
                        p { class: "mt-1 text-lg font-semibold text-paper-primary",
                            "{format_bytes(s.db_size_bytes)}"
                        }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "连接数" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary",
                            "{s.total_connections} / {s.max_connections}"
                        }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "表数量" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary",
                            "{s.tables.len()}"
                        }
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
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "行数"
                                    }
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "表大小"
                                    }
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "索引大小"
                                    }
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "总大小"
                                    }
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "死元组"
                                    }
                                }
                            }
                            tbody {
                                for t in s.tables.iter() {
                                    tr { class: "border-b border-paper-border last:border-0 hover:bg-paper-entry transition-colors",
                                        td { class: "px-4 py-2 font-mono text-paper-primary",
                                            "{t.name}"
                                        }
                                        td { class: "px-4 py-2 text-right text-paper-secondary",
                                            if t.row_count_estimated {
                                                "~{t.row_count}"
                                            } else {
                                                "{t.row_count}"
                                            }
                                        }
                                        td { class: "px-4 py-2 text-right text-paper-secondary",
                                            "{format_bytes(t.table_size_bytes)}"
                                        }
                                        td { class: "px-4 py-2 text-right text-paper-secondary",
                                            "{format_bytes(t.index_size_bytes)}"
                                        }
                                        td { class: "px-4 py-2 text-right text-paper-primary font-medium",
                                            "{format_bytes(t.total_size_bytes)}"
                                        }
                                        td { class: "px-4 py-2 text-right text-paper-secondary",
                                            "{t.dead_tuples}"
                                        }
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
                                        th { class: "px-4 py-2 font-medium text-right",
                                            "大小"
                                        }
                                    }
                                }
                                tbody {
                                    for i in s.top_indexes.iter() {
                                        tr { class: "border-b border-paper-border last:border-0 hover:bg-paper-entry transition-colors",
                                            td { class: "px-4 py-2 font-mono text-paper-primary",
                                                "{i.name}"
                                            }
                                            td { class: "px-4 py-2 font-mono text-paper-secondary",
                                                "{i.table_name}"
                                            }
                                            td { class: "px-4 py-2 text-right text-paper-secondary",
                                                "{format_bytes(i.size_bytes)}"
                                            }
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
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "时长(秒)"
                                    }
                                    th { class: "px-4 py-2 font-medium", "查询" }
                                }
                            }
                            tbody {
                                for c in s.active_connections.iter() {
                                    tr { class: "border-b border-paper-border last:border-0 hover:bg-paper-entry transition-colors",
                                        td { class: "px-4 py-2 text-paper-secondary",
                                            "{c.pid}"
                                        }
                                        td { class: "px-4 py-2 text-paper-secondary",
                                            "{c.user}"
                                        }
                                        td { class: "px-4 py-2 text-paper-secondary",
                                            {c.state.clone().unwrap_or_else(|| "—".to_string())}
                                        }
                                        td { class: "px-4 py-2 text-right text-paper-secondary",
                                            {
                                                c.query_duration_secs
                                                    .map(|d| format!("{:.1}", d))
                                                    .unwrap_or_else(|| "—".to_string())
                                            }
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
                // 首次加载骨架屏：延迟 200ms 显示，避免快速加载闪烁。
                DelayedSkeleton {
                    div { class: "space-y-4",
                        // 概览卡片骨架
                        div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                            for _ in 0..4 {
                                div { class: "rounded-2xl bg-paper-entry border border-paper-border p-4 space-y-2",
                                    SkeletonBox { class: "h-3 w-16 rounded" }
                                    SkeletonBox { class: "h-6 w-24 rounded" }
                                }
                            }
                        }
                        // 表清单骨架
                        div { class: "rounded-2xl bg-paper-entry border border-paper-border overflow-hidden",
                            div { class: "px-4 py-3 border-b border-paper-border",
                                SkeletonBox { class: "h-4 w-40 rounded" }
                            }
                            for _ in 0..5 {
                                div { class: "flex justify-between px-4 py-3 border-b border-paper-border last:border-0",
                                    SkeletonBox { class: "h-4 w-24 rounded" }
                                    SkeletonBox { class: "h-4 w-16 rounded" }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "text-paper-secondary py-8", "暂无数据" }
            }
        }
    }
}

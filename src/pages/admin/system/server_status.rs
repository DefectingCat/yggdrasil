//! 服务器状态 tab。

use dioxus::prelude::*;

use crate::components::skeletons::atoms::SkeletonBox;
use crate::components::skeletons::delayed_skeleton::DelayedSkeleton;
use crate::components::ui::LoadingButton;

use super::format_bytes;

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
pub(super) fn ServerStatusTab() -> Element {
    #[cfg(target_arch = "wasm32")]
    use crate::api::database::system_status::get_server_status;
    use crate::api::database::system_status::ServerStatus;
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
                    crate::utils::time::sleep_ms(200).await;
                    continue;
                }
                crate::utils::time::sleep_ms(ms).await;
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
                LoadingButton {
                    label: "刷新".to_string(),
                    loading: loading(),
                    variant: "sm",
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
                }
                div { class: "flex items-center gap-2",
                    span { class: "text-sm text-paper-secondary", "自动刷新" }
                    select {
                        class: "text-sm border border-paper-border rounded px-2 py-1 bg-paper-theme text-paper-primary",
                        value: "{refresh_ms().map(|s| s.to_string()).unwrap_or_default()}",
                        onchange: move |e| {
                            let v = e.value();
                            refresh_ms
                                .set(
                                    match v.as_str() {
                                        "500" => Some(500),
                                        "1000" => Some(1000),
                                        "2000" => Some(2000),
                                        "5000" => Some(5000),
                                        _ => None,
                                    },
                                );
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
                        p { class: "mt-1 text-lg font-semibold text-paper-primary",
                            "{format_uptime(s.uptime_secs)}"
                        }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "DB 连接池" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary",
                            "{s.pool_size} / {s.pool_max_size}"
                        }
                        p { class: "text-xs text-paper-secondary",
                            "空闲 {s.pool_available} · 等待 {s.pool_waiting}"
                        }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "活跃会话" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary",
                            "{s.active_sessions}"
                        }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "CPU" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary",
                            "{cpu_pct}"
                        }
                    }
                }

                // 主机层指标卡片
                div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "内存" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary",
                            "{format_bytes(s.host.used_memory as i64)} / {format_bytes(s.host.total_memory as i64)}"
                        }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "磁盘" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary",
                            "{format_bytes((s.host.disk_total - s.host.disk_available) as i64)} / {format_bytes(s.host.disk_total as i64)}"
                        }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "Load (1m)" }
                        p { class: "mt-1 text-lg font-semibold text-paper-primary",
                            "{load_1}"
                        }
                    }
                    div { class: "{ADMIN_CARD_CLASS} p-4",
                        p { class: "text-xs text-paper-secondary", "系统" }
                        p { class: "mt-1 text-sm font-medium text-paper-primary truncate",
                            "{s.host.os_name}"
                        }
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
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "条目"
                                    }
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "命中"
                                    }
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "未命中"
                                    }
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "命中率"
                                    }
                                }
                            }
                            tbody {
                                for (name, entry_count, hits, misses, rate_pct) in cache_rows.iter() {
                                    tr { class: "border-b border-paper-border last:border-0 hover:bg-paper-entry transition-colors",
                                        td { class: "px-4 py-2 text-paper-primary",
                                            "{name}"
                                        }
                                        td { class: "px-4 py-2 text-right text-paper-secondary",
                                            "{entry_count}"
                                        }
                                        td { class: "px-4 py-2 text-right text-paper-secondary",
                                            "{hits}"
                                        }
                                        td { class: "px-4 py-2 text-right text-paper-secondary",
                                            "{misses}"
                                        }
                                        td { class: "px-4 py-2 text-right text-paper-primary font-medium",
                                            "{rate_pct}"
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
                        // 应用内指标卡片骨架
                        div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                            for _ in 0..4 {
                                div { class: "rounded-2xl bg-paper-entry border border-paper-border p-4 space-y-2",
                                    SkeletonBox { class: "h-3 w-16 rounded" }
                                    SkeletonBox { class: "h-6 w-24 rounded" }
                                }
                            }
                        }
                        // 主机层指标卡片骨架
                        div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                            for _ in 0..4 {
                                div { class: "rounded-2xl bg-paper-entry border border-paper-border p-4 space-y-2",
                                    SkeletonBox { class: "h-3 w-16 rounded" }
                                    SkeletonBox { class: "h-6 w-24 rounded" }
                                }
                            }
                        }
                        // 缓存命中率表骨架
                        div { class: "rounded-2xl bg-paper-entry border border-paper-border overflow-hidden",
                            div { class: "px-4 py-3 border-b border-paper-border",
                                SkeletonBox { class: "h-4 w-24 rounded" }
                            }
                            for _ in 0..4 {
                                div { class: "flex justify-between px-4 py-3 border-b border-paper-border last:border-0",
                                    SkeletonBox { class: "h-4 w-20 rounded" }
                                    SkeletonBox { class: "h-4 w-12 rounded" }
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

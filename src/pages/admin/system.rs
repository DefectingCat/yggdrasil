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

            // tab 内容（后续 task 填充）
            div {
                match active_tab() {
                    SystemTab::DbStatus => rsx! { div { "数据库状态（待实现）" } },
                    SystemTab::ServerStatus => rsx! { div { "服务器状态（待实现）" } },
                    SystemTab::SqlConsole => rsx! { div { "SQL 控制台（待实现）" } },
                    SystemTab::Export => rsx! { div { "数据导出（待实现）" } },
                    SystemTab::Backup => rsx! { div { "备份恢复（待实现）" } },
                }
            }
        }
    }
}

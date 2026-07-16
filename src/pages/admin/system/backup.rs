//! 备份恢复 tab。

use dioxus::prelude::*;

use crate::components::ui::{LoadingButton, BTN_OUTLINE, BTN_TEXT_AMBER, BTN_TEXT_RED};

use super::format_bytes;

/// 备份恢复 tab：备份按钮 + 进度轮询 + 备份列表（下载/恢复/删除）。
#[allow(non_snake_case)]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
pub(super) fn BackupTab() -> Element {
    use crate::api::database::backup::BackupInfo;
    #[cfg(target_arch = "wasm32")]
    use crate::api::database::backup::{
        create_backup, delete_backup, list_backups, restore_backup,
    };
    use crate::api::database::tasks::TaskProgress;
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

    // 任务进度轮询：active_task_id 存在时每 1.5s 拉取进度，Done/Failed 后停止 + 刷新列表。
    //
    // 同样用长生命周期 loop + 循环内读 active_task_id() 的模式。原先在挂载时把
    // active_task_id 快照进 _task_id_for_poll（彼时为 None），use_future 只跑一次
    // 即 return；用户点"创建备份"后 create_backup 返回 task id 并设置信号，但
    // future 已结束 → 轮询永不启动，busy 永远为 true（用户报告的 bug）。
    use_future(move || {
        let mut active_task_id = active_task_id;
        let mut active_progress = active_progress;
        let mut backups_f = backups;
        let mut busy_f = busy;
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                loop {
                    let tid = match active_task_id() {
                        Some(t) => t,
                        None => {
                            // 空闲：短 yield，最多 200ms 后响应新任务。
                            crate::utils::time::sleep_ms(200).await;
                            continue;
                        }
                    };
                    // 有任务在途：进入 1.5s 轮询，直到 Done/Failed/出错。
                    loop {
                        crate::utils::time::sleep_ms(1500).await;
                        match get_task_progress(tid.clone()).await {
                            Ok(p) => {
                                let done =
                                    p.status == TaskStatus::Done || p.status == TaskStatus::Failed;
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
                    // 内层 loop 退出后回到外层，继续等待下一个任务或空闲。
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (active_task_id, active_progress, backups_f, busy_f);
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
                LoadingButton {
                    label: "创建备份".to_string(),
                    loading: is_busy,
                    variant: "sm",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            busy.set(true);
                            active_progress.set(None);
                            let mut active_task_id = active_task_id;
                            spawn(async move {
                                match create_backup().await {
                                    Ok(id) => active_task_id.set(Some(id)),
                                    Err(e) => {
                                        error.set(Some(e.to_string()));
                                        busy.set(false);
                                    }
                                }
                            });
                        }
                    },
                }
                button {
                    class: "{BTN_OUTLINE}",
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
                        div {
                            class: "bg-paper-accent h-full transition-all",
                            style: "width: {p.percent}%",
                        }
                    }
                    if let Some(detail) = p.detail {
                        p { class: "text-xs text-paper-secondary mt-2", "{detail}" }
                    }
                    if let Some(err) = p.error {
                        p { class: "text-xs text-red-600 dark:text-red-400 mt-2",
                            "错误：{err}"
                        }
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
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "大小"
                                    }
                                    th { class: "px-4 py-2 font-medium text-right",
                                        "操作"
                                    }
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
                                        // 恢复：确认已在 BackupRow 的 Popover 内完成,
                                        // 这里直接发起 restore_backup 并交由轮询 use_future 接管。
                                        // pending_restore signal + 确认 use_future 链路已移除
                                        //（原生 confirm 是阻塞式才需要那套间接机制）。
                                        on_restore: move |f: String| {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let mut busy = busy;
                                                let mut active_progress = active_progress;
                                                let mut active_task_id = active_task_id;
                                                let mut error = error;
                                                spawn(async move {
                                                    busy.set(true);
                                                    active_progress.set(None);
                                                    match restore_backup(f, true).await {
                                                        Ok(id) => active_task_id.set(Some(id)),
                                                        Err(e) => {
                                                            error.set(Some(e.to_string()));
                                                            busy.set(false);
                                                        }
                                                    }
                                                });
                                            }
                                        },
                                        // 删除:确认已在 BackupRow 的 Popover 内完成,
                                        // 直接执行 delete_backup + 刷新列表。
                                        on_delete: move |fname_del: String| {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let mut backups = backups;
                                                spawn(async move {
                                                    let _ = delete_backup(fname_del).await;
                                                    if let Ok(list) = list_backups().await {
                                                        backups.set(list);
                                                    }
                                                });
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
/// 下载链接用的 URL 编码（wasm32 才编码，server 端原样返回——rsx 构造 dl_url 时两端都调）。
/// 自包含实现，不跨文件依赖 export.rs 的 urlencode。
fn urlencode_dl(s: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        s.to_string()
    }
}
/// 备份列表单行（抽取为子组件：各自 scope 内 let/clone 不冲突）。
///
/// 删除/恢复不再用浏览器原生 confirm()，改用 [`Popover`](crate::components::ui::Popover) 确认框（`position:fixed`
/// 逃出表格 `overflow-hidden`）。点击按钮读 `MouseEvent::client_coordinates()` 作为
/// popover 锚点，`confirm` 按钮回调父组件的 `on_delete`/`on_restore`。
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
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut, unused_variables))]
fn BackupRow(props: BackupRowProps) -> Element {
    use crate::components::ui::Popover;
    use crate::components::ui::BTN_DANGER_OUTLINE;

    // Callback 是 Copy，直接复用；filename 需 clone（确认框闭包各取一份）。
    let on_restore = props.on_restore;
    let on_delete = props.on_delete;
    let fname_for_restore = props.filename.clone();
    let fname_for_delete = props.filename.clone();

    // Popover 状态：哪个动作的确认框打开 + 锚点坐标。None = 都关闭。
    // 用一个 String("delete"/"restore") 而非两个 bool，避免同时开两个 popover。
    let mut open_action = use_signal(|| Option::<String>::None);
    // 锚点坐标：按钮点击的视口坐标（client_coordinates）。
    let mut anchor_x = use_signal(|| 0i32);
    let mut anchor_y = use_signal(|| 0i32);

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
                    class: "{BTN_TEXT_AMBER} mr-3 disabled:opacity-50",
                    disabled: props.busy,
                    // 点击记录坐标并打开恢复确认 popover。client_coordinates 两端编译。
                    onclick: move |e| {
                        let c = e.client_coordinates();
                        anchor_x.set(c.x as i32);
                        anchor_y.set(c.y as i32);
                        open_action.set(Some("restore".to_string()));
                    },
                    "恢复"
                }
                button {
                    class: "{BTN_TEXT_RED} disabled:opacity-50",
                    disabled: props.busy,
                    onclick: move |e| {
                        let c = e.client_coordinates();
                        anchor_x.set(c.x as i32);
                        anchor_y.set(c.y as i32);
                        open_action.set(Some("delete".to_string()));
                    },
                    "删除"
                }
            }

            // 恢复确认 popover
            Popover {
                open: open_action().as_deref() == Some("restore"),
                anchor_x: anchor_x(),
                anchor_y: anchor_y(),
                placement: "bottom",
                on_close: move |_| open_action.set(None),
                div { class: "w-64 space-y-3",
                    p { class: "text-sm text-paper-primary leading-relaxed",
                        "恢复将覆盖现有数据，确认恢复 "
                        span { class: "font-mono text-xs break-all", "{props.filename}" }
                        "？"
                    }
                    p { class: "text-xs text-paper-secondary",
                        "仅本系统生成的备份可恢复。"
                    }
                    div { class: "flex justify-end gap-2 pt-1",
                        button {
                            class: "px-3 py-1.5 text-xs text-paper-secondary hover:text-paper-primary transition-colors cursor-pointer",
                            onclick: move |_| open_action.set(None),
                            "取消"
                        }
                        button {
                            class: "{BTN_DANGER_OUTLINE}",
                            onclick: move |_| {
                                open_action.set(None);
                                on_restore.call(fname_for_restore.clone());
                            },
                            "确认恢复"
                        }
                    }
                }
            }

            // 删除确认 popover
            Popover {
                open: open_action().as_deref() == Some("delete"),
                anchor_x: anchor_x(),
                anchor_y: anchor_y(),
                placement: "bottom",
                on_close: move |_| open_action.set(None),
                div { class: "w-64 space-y-3",
                    p { class: "text-sm text-paper-primary",
                        "确认删除 "
                        span { class: "font-mono text-xs break-all", "{props.filename}" }
                        "？"
                    }
                    div { class: "flex justify-end gap-2 pt-1",
                        button {
                            class: "px-3 py-1.5 text-xs text-paper-secondary hover:text-paper-primary transition-colors cursor-pointer",
                            onclick: move |_| open_action.set(None),
                            "取消"
                        }
                        button {
                            class: "{BTN_DANGER_OUTLINE}",
                            onclick: move |_| {
                                open_action.set(None);
                                on_delete.call(fname_for_delete.clone());
                            },
                            "确认删除"
                        }
                    }
                }
            }
        }
    }
}

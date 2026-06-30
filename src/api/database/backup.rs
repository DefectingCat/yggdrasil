#![allow(clippy::unused_unit, deprecated)]

//! 备份与恢复（读写，最高风险）。
//!
//! 备份：探测 pg_dump 可用性——可用则子进程生成完整 .sql，不可用则回退纯 SQL
//! （仅数据）。备份文件含签名头。
//! 恢复：仅接受本系统生成的备份（签名校验）+ 二次确认 + 路径穿越防护。
//! 长耗时操作走后台任务 + 进度轮询（见 [`crate::api::database::tasks`]）。

// Component/PathBuf/chrono::Utc 仅 server 构建的备份逻辑用到。
#[cfg(feature = "server")]
use std::path::{Component, PathBuf};

#[cfg(feature = "server")]
use chrono::Utc;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

// admin 鉴权 + AppError + tasks 进度表仅在 server 构建里被 server function 体引用。
#[cfg(feature = "server")]
use crate::api::auth::get_current_admin_user;
#[cfg(feature = "server")]
use crate::api::database::tasks::{self, TaskKind, TaskStatus};
#[cfg(feature = "server")]
use crate::api::error::AppError;

// 以下常量仅被 server 构建的备份/恢复逻辑引用（WASM 构建里相关函数体被 cfg 剥掉，
// 故常量也需 gate，否则非 server 构建会报 dead_code）。

/// 备份目录（项目根，与 uploads/ 平级，gitignored）。
#[cfg(feature = "server")]
const BACKUP_DIR: &str = "backups";
/// 文件名白名单正则：仅字母数字下划线点连字符（防路径穿越）。
#[cfg(feature = "server")]
const FILENAME_RE: &str = r"^[a-zA-Z0-9_.\-]+$";
/// 备份文件签名头（恢复时校验，拒绝非本系统文件）。
#[cfg(feature = "server")]
const BACKUP_SIGNATURE: &str = "-- YGGDRASIL BACKUP v1";

/// 备份文件元信息（列表展示用）。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupInfo {
    pub filename: String,
    pub size_bytes: u64,
    /// 备份模式：pg_dump / sql-fallback（从签名头解析）。
    pub mode: String,
    pub created_at: Option<String>,
}

/// 发起备份，立即返回 task_id，后台任务执行。
#[server(CreateBackup, "/api")]
pub async fn create_backup() -> Result<String, ServerFnError> {
    let _user = get_current_admin_user().await?;

    #[cfg(feature = "server")]
    {
        let task_id = uuid::Uuid::new_v4().to_string();
        tasks::insert(task_id.clone(), TaskKind::Backup);
        let tid = task_id.clone();
        tokio::spawn(async move {
            run_backup(&tid).await;
        });
        Ok(task_id)
    }
    #[cfg(not(feature = "server"))]
    {
        Ok(String::new())
    }
}

/// 后台执行备份：pg_dump 优先，不可用回退纯 SQL。
#[cfg(feature = "server")]
async fn run_backup(task_id: &str) {
    let _ = std::fs::create_dir_all(BACKUP_DIR);
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();

    // 探测 pg_dump
    let pg_dump_ok = std::process::Command::new("pg_dump")
        .arg("--version")
        .output()
        .is_ok();

    if pg_dump_ok {
        run_pg_dump_backup(task_id, &timestamp).await;
    } else {
        run_sql_fallback_backup(task_id, &timestamp).await;
    }
}

/// pg_dump 模式：子进程生成完整备份（含 schema），前置签名头。
#[cfg(feature = "server")]
async fn run_pg_dump_backup(task_id: &str, timestamp: &str) {
    tasks::update(
        task_id,
        "正在用 pg_dump 导出",
        10,
        TaskStatus::Running,
        None,
        None,
        None,
    );
    let filename = format!("backup_{}.sql", timestamp);
    let path = backup_path(&filename);
    let db_url = match std::env::var("DATABASE_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => {
            tasks::update(
                task_id,
                "DATABASE_URL 未配置",
                100,
                TaskStatus::Failed,
                None,
                Some("pg_dump 备份需要 DATABASE_URL".to_string()),
                None,
            );
            return;
        }
    };

    let mut header = String::new();
    header.push_str(&format!("{}\n", BACKUP_SIGNATURE));
    header.push_str(&format!("-- created_at: {}\n", Utc::now()));
    header.push_str("-- mode: pg_dump\n");

    // 先写签名头，再追加 pg_dump 输出。
    let write_header = std::fs::write(&path, &header);
    if write_header.is_err() {
        tasks::update(
            task_id,
            "写入备份文件失败",
            100,
            TaskStatus::Failed,
            None,
            Some("无法写入备份目录".to_string()),
            None,
        );
        return;
    }

    let stdout_file = match std::fs::OpenOptions::new().append(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            tasks::update(
                task_id,
                "pg_dump 启动失败",
                100,
                TaskStatus::Failed,
                None,
                Some(e.to_string()),
                None,
            );
            return;
        }
    };
    let child = match std::process::Command::new("pg_dump")
        .arg(&db_url)
        .stdout(std::process::Stdio::from(stdout_file))
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tasks::update(
                task_id,
                "pg_dump 启动失败",
                100,
                TaskStatus::Failed,
                None,
                Some(e.to_string()),
                None,
            );
            return;
        }
    };
    let output = child.wait_with_output();
    match output {
        Ok(o) if o.status.success() => {
            tasks::update(
                task_id,
                "完成",
                100,
                TaskStatus::Done,
                None,
                None,
                Some(filename),
            );
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            tasks::update(
                task_id,
                "pg_dump 失败",
                100,
                TaskStatus::Failed,
                None,
                Some(stderr),
                None,
            );
        }
        Err(e) => {
            tasks::update(
                task_id,
                "pg_dump 执行失败",
                100,
                TaskStatus::Failed,
                None,
                Some(e.to_string()),
                None,
            );
        }
    }
}

/// 纯 SQL 回退：仅备份数据（不含 schema），按表计数精确进度。
#[cfg(feature = "server")]
async fn run_sql_fallback_backup(task_id: &str, timestamp: &str) {
    tasks::update(
        task_id,
        "pg_dump 不可用，使用纯 SQL 回退（仅数据）",
        10,
        TaskStatus::Running,
        Some("仅备份数据，不含 schema/索引/触发器".to_string()),
        None,
        None,
    );
    let filename = format!("backup_{}_sqlfallback.sql", timestamp);
    let path = backup_path(&filename);

    let client = match crate::db::pool::get_conn().await {
        Ok(c) => c,
        Err(e) => {
            tasks::update(
                task_id,
                "数据库连接失败",
                100,
                TaskStatus::Failed,
                None,
                Some(e.to_string()),
                None,
            );
            return;
        }
    };

    // 取 public schema 下所有表名
    let tables: Vec<String> = match client
        .query(
            "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename",
            &[],
        )
        .await
    {
        Ok(rows) => rows.into_iter().map(|r| r.get(0)).collect(),
        Err(e) => {
            tasks::update(
                task_id,
                "读取表清单失败",
                100,
                TaskStatus::Failed,
                None,
                Some(e.to_string()),
                None,
            );
            return;
        }
    };
    let total = tables.len().max(1);

    let mut out = String::new();
    out.push_str(&format!("{}\n", BACKUP_SIGNATURE));
    out.push_str(&format!("-- created_at: {}\n", Utc::now()));
    out.push_str("-- mode: sql-fallback\n\n");

    for (i, table) in tables.iter().enumerate() {
        out.push_str(&format!("\n-- table: {}\n", table));
        let copy_stmt = format!("COPY \"{}\" TO STDOUT WITH CSV", table);
        match client.copy_out(&copy_stmt).await {
            Ok(stream) => {
                use futures::StreamExt;
                // CopyOutStream 是 !Unpin，必须 pin 才能调 next。
                tokio::pin!(stream);
                while let Some(chunk) = stream.next().await {
                    if let Ok(bytes) = chunk {
                        out.push_str(&String::from_utf8_lossy(&bytes));
                    }
                }
            }
            Err(e) => {
                out.push_str(&format!("-- 导出失败: {}\n", e));
            }
        }
        // 按表更新进度（用 u32 避免大 schema 下的截断/溢出）
        tasks::update(
            task_id,
            &format!("导出表 {}/{}", i + 1, total),
            (10 + (i + 1) as u32 * 90 / total as u32).min(99) as u8,
            TaskStatus::Running,
            None,
            None,
            None,
        );
    }

    if std::fs::write(&path, out).is_err() {
        tasks::update(
            task_id,
            "写入备份文件失败",
            100,
            TaskStatus::Failed,
            None,
            Some("无法写入备份目录".to_string()),
            None,
        );
        return;
    }
    tasks::update(
        task_id,
        "完成",
        100,
        TaskStatus::Done,
        None,
        None,
        Some(filename),
    );
}

/// 发起恢复：校验签名 + 路径穿越防护 + 二次确认，立即返回 task_id。
#[server(RestoreBackup, "/api")]
pub async fn restore_backup(filename: String, confirm: bool) -> Result<String, ServerFnError> {
    let _user = get_current_admin_user().await?;

    // 全部校验都在 server cfg 块内：confirm/regex/backup_path/std::fs 都是 server-only。
    // WASM 侧的 server-function 客户端桩只返回 Ok(String::new())。
    #[cfg(feature = "server")]
    {
        if !confirm {
            return Err(AppError::BadRequest("需确认恢复（会覆盖现有数据）".to_string()).into());
        }
        // 路径穿越防护
        let re = regex::Regex::new(FILENAME_RE).unwrap();
        if !re.is_match(&filename) {
            return Err(AppError::BadRequest("无效的文件名".to_string()).into());
        }
        let path = backup_path(&filename);
        if !path.exists() {
            return Err(AppError::NotFound("备份文件不存在").into());
        }

        // 签名校验：首行需含签名
        let head = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
            .unwrap_or_default();
        if !head.contains(BACKUP_SIGNATURE) {
            return Err(AppError::BadRequest("非本系统生成的备份文件，拒绝恢复".to_string()).into());
        }

        let task_id = uuid::Uuid::new_v4().to_string();
        tasks::insert(task_id.clone(), TaskKind::Restore);
        let tid = task_id.clone();
        let f = filename;
        tokio::spawn(async move {
            run_restore(&tid, &f).await;
        });
        Ok(task_id)
    }
    #[cfg(not(feature = "server"))]
    {
        // WASM 客户端桩：忽略参数，返回空 task_id。
        let _ = (filename, confirm);
        Ok(String::new())
    }
}

/// 后台执行恢复：探测 psql，可用则 psql -f，不可用则报告。
#[cfg(feature = "server")]
async fn run_restore(task_id: &str, filename: &str) {
    let path = backup_path(filename);
    let db_url = match std::env::var("DATABASE_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => {
            tasks::update(
                task_id,
                "DATABASE_URL 未配置",
                100,
                TaskStatus::Failed,
                None,
                Some("恢复需要 DATABASE_URL".to_string()),
                None,
            );
            return;
        }
    };
    let psql_ok = std::process::Command::new("psql")
        .arg("--version")
        .output()
        .is_ok();
    if !psql_ok {
        tasks::update(
            task_id,
            "psql 不可用",
            100,
            TaskStatus::Failed,
            None,
            Some("恢复需要 psql，但当前环境未安装 psql".to_string()),
            None,
        );
        return;
    }
    tasks::update(
        task_id,
        "正在用 psql 恢复",
        50,
        TaskStatus::Running,
        None,
        None,
        None,
    );
    let output = std::process::Command::new("psql")
        .arg(&db_url)
        .arg("-f")
        .arg(&path)
        .stderr(std::process::Stdio::piped())
        .output();
    match output {
        Ok(o) if o.status.success() => {
            tasks::update(task_id, "恢复完成", 100, TaskStatus::Done, None, None, None);
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            tasks::update(
                task_id,
                "恢复失败",
                100,
                TaskStatus::Failed,
                None,
                Some(stderr),
                None,
            );
        }
        Err(e) => {
            tasks::update(
                task_id,
                "psql 启动失败",
                100,
                TaskStatus::Failed,
                None,
                Some(e.to_string()),
                None,
            );
        }
    }
}

/// 列出 backups/ 目录下的备份文件元信息。
#[server(ListBackups, "/api")]
pub async fn list_backups() -> Result<Vec<BackupInfo>, ServerFnError> {
    let _user = get_current_admin_user().await?;
    #[cfg(feature = "server")]
    {
        let mut infos: Vec<BackupInfo> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(BACKUP_DIR) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with(".sql") {
                    continue;
                }
                let meta = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let mode = std::fs::read_to_string(entry.path())
                    .ok()
                    .and_then(|s| {
                        s.lines()
                            .find(|l| l.starts_with("-- mode:"))
                            .map(|l| l.trim_start_matches("-- mode: ").trim().to_string())
                    })
                    .unwrap_or_else(|| "unknown".to_string());
                let created_at = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| {
                        chrono::DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default()
                    });
                infos.push(BackupInfo {
                    filename: name,
                    size_bytes: meta.len(),
                    mode,
                    created_at,
                });
            }
        }
        // 按创建时间降序（新的在前）
        infos.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(infos)
    }
    #[cfg(not(feature = "server"))]
    {
        Ok(vec![])
    }
}

/// 删除备份文件。
#[server(DeleteBackup, "/api")]
pub async fn delete_backup(filename: String) -> Result<(), ServerFnError> {
    let _user = get_current_admin_user().await?;
    #[cfg(feature = "server")]
    {
        let re = regex::Regex::new(FILENAME_RE).unwrap();
        if !re.is_match(&filename) {
            return Err(AppError::BadRequest("无效的文件名".to_string()).into());
        }
        let path = backup_path(&filename);
        if !path.exists() {
            return Err(AppError::NotFound("备份文件不存在").into());
        }
        std::fs::remove_file(&path).map_err(|_| AppError::Internal("删除失败"))?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    {
        Ok(())
    }
}

/// 构造 backups/ 下的安全路径（额外防御：校验规范化后仍在 BACKUP_DIR 内）。
#[cfg(feature = "server")]
fn backup_path(filename: &str) -> PathBuf {
    let raw: PathBuf = [BACKUP_DIR, filename].iter().collect();
    // 确保规范化后首两段仍是 BACKUP_DIR/filename（防任何路径穿越残留）
    let mut it = raw.components();
    let _ = it.next(); // BACKUP_DIR
    if it.all(|c| !matches!(c, Component::ParentDir | Component::RootDir)) {
        raw
    } else {
        PathBuf::from(BACKUP_DIR)
    }
}

/// Axum 处理器：下载备份文件（admin 鉴权 + 路径白名单）。
/// 仅 server 构建：纯 Axum 路由（在 main.rs 注册），无 WASM 消费者。
#[cfg(feature = "server")]
pub async fn download_backup(
    axum::extract::Path(filename): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
) -> Result<impl axum::response::IntoResponse, (axum::http::StatusCode, String)> {
    use axum::http::{header, StatusCode};

    // 鉴权
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let token = crate::auth::session::parse_session_token(cookie_header).map(str::to_string);
    let token = match token {
        Some(t) => t,
        None => return Err((StatusCode::UNAUTHORIZED, "未登录".to_string())),
    };
    let user = match crate::api::auth::get_user_by_token(&token).await {
        Ok(Some(u)) => u,
        _ => return Err((StatusCode::UNAUTHORIZED, "会话已过期".to_string())),
    };
    if user.role != crate::models::user::UserRole::Admin {
        return Err((StatusCode::FORBIDDEN, "权限不足".to_string()));
    }

    // 路径白名单
    let re = regex::Regex::new(FILENAME_RE).unwrap();
    if !re.is_match(&filename) {
        return Err((StatusCode::BAD_REQUEST, "无效的文件名".to_string()));
    }
    let path = backup_path(&filename);
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "文件不存在".to_string()))?;
    let disposition = format!("attachment; filename=\"{}\"", filename);
    Ok((
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/sql; charset=utf-8"),
            ),
            (
                header::CONTENT_DISPOSITION,
                axum::http::HeaderValue::from_str(&disposition)
                    .unwrap_or_else(|_| axum::http::HeaderValue::from_static("attachment")),
            ),
        ],
        axum::body::Body::from(bytes),
    ))
}

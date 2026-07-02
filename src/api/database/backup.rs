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
        if !is_valid_backup_filename(&filename) {
            return Err(AppError::BadRequest("无效的文件名".to_string()).into());
        }
        let path = backup_path(&filename);
        if !path.exists() {
            return Err(AppError::NotFound("备份文件不存在").into());
        }

        // 签名校验：首行需含签名
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        if !has_valid_signature(&content) {
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
                    .map(|s| parse_backup_mode(&s))
                    .unwrap_or_else(|_| "unknown".to_string());
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
        if !is_valid_backup_filename(&filename) {
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
///
/// 纵深防御：即便第一道白名单 `is_valid_backup_filename` 被绕过，这里也要
/// 保证结果不逃出 BACKUP_DIR。直接对 filename 做 components 检查——
/// 含 `..`（ParentDir）、绝对路径前缀（RootDir/Prefix，如 `/etc` 或 `C:\`）
/// 的 filename 一律降级为 BACKUP_DIR 本身。
///
/// 注意：不能用 `[BACKUP_DIR, filename].collect::<PathBuf>()` 后再检——
/// 当 filename 是绝对路径时，PathBuf 语义会丢弃 BACKUP_DIR 前缀（如
/// `["backups", "/etc/passwd"]` → `/etc/passwd`），导致 components 检查
/// 在错位的路径上运行而漏判。必须先检 filename 本身。
#[cfg(feature = "server")]
fn backup_path(filename: &str) -> PathBuf {
    // 直接检查 filename 的 components：只允许 Normal 段。
    let filename_is_safe = std::path::Path::new(filename)
        .components()
        .all(|c| matches!(c, Component::Normal(_)));
    if filename_is_safe {
        let mut p = PathBuf::from(BACKUP_DIR);
        p.push(filename);
        p
    } else {
        // 命中 ParentDir/RootDir/Prefix/CurDir → 降级为 BACKUP_DIR
        PathBuf::from(BACKUP_DIR)
    }
}

/// 校验备份文件名是否符合白名单（仅字母数字下划线点连字符）。
/// 返回 true 表示安全可用。提取为纯函数便于单测覆盖路径穿越边界。
#[cfg(feature = "server")]
fn is_valid_backup_filename(filename: &str) -> bool {
    // regex::Regex::new 在 FILENAME_RE 是常量正则,编译期可验证不会 panic。
    regex::Regex::new(FILENAME_RE)
        .map(|re| re.is_match(filename))
        .unwrap_or(false)
}

/// 从备份文件全文提取 `-- mode: <value>` 行的值（如 "pg_dump"/"sql-fallback"）。
/// 提取为纯函数:把文件内容作为参数传入,便于单测。
/// 缺失或格式不符返回 "unknown"。
#[cfg(feature = "server")]
fn parse_backup_mode(content: &str) -> String {
    content
        .lines()
        .find(|l| l.starts_with("-- mode:"))
        .map(|l| l.trim_start_matches("-- mode:").trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// 校验备份文件首行是否含本系统签名头。
/// 提取为纯函数:把首行(或全文)作为参数传入,便于单测。
#[cfg(feature = "server")]
fn has_valid_signature(content: &str) -> bool {
    content
        .lines()
        .next()
        .map(|l| l.trim().contains(BACKUP_SIGNATURE))
        .unwrap_or(false)
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
    if !is_valid_backup_filename(&filename) {
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

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    // ── is_valid_backup_filename:文件名白名单(路径穿越第一道防线) ──

    #[test]
    fn filename_accepts_normal_names() {
        for name in [
            "backup_20260702_120000.sql",
            "backup_20260702_120000_sqlfallback.sql",
            "a.sql",
            "A-B_C.123",
        ] {
            assert!(
                is_valid_backup_filename(name),
                "正常文件名应通过: {name}"
            );
        }
    }

    #[test]
    fn filename_rejects_path_traversal() {
        // 路径穿越:白名单只允许字母数字下划线点连字符,/ 和 .. 都应被拒。
        for evil in [
            "../etc/passwd",
            "..\\windows\\win.ini",
            "/etc/passwd",
            "a/../../b",
            "backup.sql/../../etc",
        ] {
            assert!(
                !is_valid_backup_filename(evil),
                "路径穿越应被拒: {evil}"
            );
        }
    }

    #[test]
    fn filename_rejects_spaces_and_special_chars() {
        // 空格、中文、shell 元字符等都不在白名单。
        for evil in [
            "backup with space.sql",
            "备份.sql",
            "a;rm -rf.sql",
            r"a\$b.sql",
            "a`b`.sql",
            "",
        ] {
            assert!(
                !is_valid_backup_filename(evil),
                "特殊字符应被拒: {evil:?}"
            );
        }
    }

    // ── backup_path:路径穿越纵深防御(白名单之外的二次防御) ────────

    #[test]
    fn backup_path_stays_in_backup_dir_for_normal_name() {
        let p = backup_path("backup_20260702.sql");
        assert!(p.starts_with(BACKUP_DIR), "应在 {BACKUP_DIR}/ 下");
        assert_eq!(p.file_name().and_then(|n| n.to_str()), Some("backup_20260702.sql"));
    }

    #[test]
    fn backup_path_collapses_traversal_to_backup_dir() {
        // 即便绕过白名单调用 backup_path(纵深防御),../ 也应被规约回 BACKUP_DIR,
        // 而非指向 backups/ 之外。Component::ParentDir / RootDir 命中即降级。
        for evil in ["../etc/passwd", "../../etc/shadow"] {
            let p = backup_path(evil);
            // 不应逃出 BACKUP_DIR(应为 BACKUP_DIR 本身,不含文件名)
            assert_eq!(
                p, PathBuf::from(BACKUP_DIR),
                "穿越应被规约回 {BACKUP_DIR}: {evil}"
            );
        }
    }

    #[test]
    fn backup_path_rejects_absolute_path() {
        // Component::RootDir 命中也应降级。
        let p = backup_path("/etc/passwd");
        assert_eq!(p, PathBuf::from(BACKUP_DIR));
    }

    // ── has_valid_signature:备份签名校验(拒绝非本系统文件) ───────

    #[test]
    fn signature_matches_exact_header() {
        let content = "-- YGGDRASIL BACKUP v1\n-- mode: pg_dump\nSELECT 1;\n";
        assert!(has_valid_signature(content));
    }

    #[test]
    fn signature_matches_with_leading_whitespace() {
        // 首行允许前导空白(trim 后匹配),容忍编辑器缩进。
        let content = "  -- YGGDRASIL BACKUP v1\nrest\n";
        assert!(has_valid_signature(content));
    }

    #[test]
    fn signature_rejects_non_system_file() {
        // 普通 SQL 文件首行不含签名 → 拒绝恢复(防任意文件读取/执行)。
        let content = "SELECT * FROM users;\n-- YGGDRASIL BACKUP v1\n";
        // 注意:签名必须在首行。第二行有签名不算。
        assert!(!has_valid_signature(content));
    }

    #[test]
    fn signature_rejects_empty_and_garbage() {
        assert!(!has_valid_signature(""));
        assert!(!has_valid_signature("garbage\n"));
        assert!(!has_valid_signature("\n\n-- YGGDRASIL BACKUP v1"));
    }

    // ── parse_backup_mode:模式解析(列表展示用) ───────────────────

    #[test]
    fn parse_mode_pg_dump() {
        let content = "-- YGGDRASIL BACKUP v1\n-- mode: pg_dump\n...\n";
        assert_eq!(parse_backup_mode(content), "pg_dump");
    }

    #[test]
    fn parse_mode_sql_fallback() {
        let content = "-- YGGDRASIL BACKUP v1\n-- mode: sql-fallback\n\n-- table: posts\n";
        assert_eq!(parse_backup_mode(content), "sql-fallback");
    }

    #[test]
    fn parse_mode_unknown_when_absent() {
        let content = "-- YGGDRASIL BACKUP v1\nSELECT 1;\n";
        assert_eq!(parse_backup_mode(content), "unknown");
    }

    #[test]
    fn parse_mode_unknown_when_empty_value() {
        // "-- mode:" 后无值 → unknown(防空字符串显示)
        let content = "-- mode:\nrest\n";
        assert_eq!(parse_backup_mode(content), "unknown");
    }

    #[test]
    fn parse_mode_only_matches_first_occurrence() {
        // 多个 -- mode: 行取第一个。
        let content = "-- mode: pg_dump\n-- mode: sql-fallback\n";
        assert_eq!(parse_backup_mode(content), "pg_dump");
    }
}

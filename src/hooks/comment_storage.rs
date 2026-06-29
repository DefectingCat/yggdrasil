//! 评论草稿在浏览器 localStorage 中的持久化支持。
//!
//! 注意：所有 localStorage 读写均通过 `#[cfg(target_arch = "wasm32")]` 限定，
//! 仅在 WASM 前端生效；服务端渲染（SSR）路径下这些函数会返回 None 或空操作。

use chrono::DateTime;
use serde::{Deserialize, Serialize};

/// localStorage 中用于存储评论作者信息的键名。
const AUTHOR_KEY: &str = "yggdrasil-comment-author";

/// localStorage 中用于存储待发布评论草稿的键名。
const PENDING_KEY: &str = "yggdrasil-pending-comments";

/// 待发布评论草稿的过期时间（天）。
const TTL_DAYS: i64 = 7;

/// 评论作者信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    /// 昵称。
    pub name: String,
    /// 邮箱地址。
    pub email: String,
    /// 个人主页 URL。
    #[serde(default)]
    pub url: String,
}

/// 待发布评论草稿。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingComment {
    /// 评论 ID。
    pub id: i64,
    /// 父评论 ID，顶级评论为 None。
    pub parent_id: Option<i64>,
    /// 评论层级深度。
    pub depth: i32,
    /// 作者昵称。
    pub author_name: String,
    /// 作者主页 URL。
    pub author_url: Option<String>,
    /// 头像 URL。
    pub avatar_url: String,
    /// Markdown 格式的评论内容。
    pub content_md: String,
    /// 评论创建时间（RFC3339 字符串）。
    pub created_at: String,
    /// 草稿存入 localStorage 的时间（RFC3339 字符串）。
    pub stored_at: String,
}

/// 按文章 ID 组织的待发布评论草稿映射。
type PendingMap = std::collections::HashMap<String, Vec<PendingComment>>;

/// 从 localStorage 读取指定键的值。
///
/// 仅在 `wasm32` 目标下执行实际读取；SSR 构建下直接返回 None。
#[allow(unused_variables)]
fn read_storage(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let storage = window.local_storage().ok()??;
        storage.get_item(key).ok()?
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// 将值写入 localStorage 指定键。
///
/// 仅在 `wasm32` 目标下执行实际写入；SSR 构建下为空操作。
#[allow(unused_variables)]
fn write_storage(key: &str, value: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(key, value);
            }
        }
    }
}

/// 获取当前时间戳（毫秒）。
///
/// WASM 端使用 `js_sys::Date`，服务端回退到 `chrono::Utc`。
fn now_millis() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as i64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        chrono::Utc::now().timestamp_millis()
    }
}

/// 判断给定存储时间是否已经过期。
fn is_expired(stored_at: &str) -> bool {
    let Ok(dt) = DateTime::parse_from_rfc3339(stored_at) else {
        return true;
    };
    let now_ms = now_millis();
    let stored_ms = dt.timestamp_millis();
    (now_ms - stored_ms) > (TTL_DAYS * 24 * 60 * 60 * 1000)
}

/// 保存评论作者信息到 localStorage。
pub fn save_author(name: &str, email: &str, url: &str) {
    let info = AuthorInfo {
        name: name.to_string(),
        email: email.to_string(),
        url: url.to_string(),
    };
    if let Ok(json) = serde_json::to_string(&info) {
        write_storage(AUTHOR_KEY, &json);
    }
}

/// 从 localStorage 读取评论作者信息。
pub fn load_author() -> Option<AuthorInfo> {
    let json = read_storage(AUTHOR_KEY)?;
    serde_json::from_str(&json).ok()
}

/// 将一条待发布评论草稿保存到指定文章的草稿列表中。
///
/// 如果同一 ID 的草稿已存在则忽略，避免重复。
pub fn save_pending_comment(post_id: i32, comment: PendingComment) {
    let mut map: PendingMap = load_all_pending();
    let key = post_id.to_string();
    let list = map.entry(key).or_default();

    // 已存在相同 ID 时直接返回，避免重复保存。
    if list.iter().any(|c| c.id == comment.id) {
        return;
    }
    list.push(comment);

    if let Ok(json) = serde_json::to_string(&map) {
        write_storage(PENDING_KEY, &json);
    }
}

/// 加载指定文章下所有未过期的待发布评论草稿。
///
/// 读取时会自动清理过期草稿，并将结果写回 localStorage。
pub fn load_pending_comments(post_id: i32) -> Vec<PendingComment> {
    let mut map = load_all_pending();
    let key = post_id.to_string();

    let comments = map.remove(&key).unwrap_or_default();
    let original_len = comments.len();
    let non_expired: Vec<PendingComment> = comments
        .into_iter()
        .filter(|c| !is_expired(&c.stored_at))
        .collect();

    // 若有草稿被清理，或该文章已无草稿，都需要把更新后的映射写回 localStorage。
    let pruned = non_expired.len() != original_len;
    if !non_expired.is_empty() {
        map.insert(key, non_expired.clone());
    }
    if pruned || non_expired.is_empty() {
        if let Ok(json) = serde_json::to_string(&map) {
            write_storage(PENDING_KEY, &json);
        }
    }

    non_expired
}

/// 从指定文章的草稿列表中移除指定 ID 的评论。
///
/// 若移除后该文章无草稿，则删除该文章对应的键。
pub fn remove_pending_ids(post_id: i32, ids: &[i64]) {
    let mut map = load_all_pending();
    let key = post_id.to_string();

    let should_remove = if let Some(comments) = map.get_mut(&key) {
        comments.retain(|c| !ids.contains(&c.id));
        comments.is_empty()
    } else {
        false
    };
    if should_remove {
        map.remove(&key);
    }

    if let Ok(json) = serde_json::to_string(&map) {
        write_storage(PENDING_KEY, &json);
    }
}

/// 清理所有文章中已过期的待发布评论草稿。
pub fn prune_all_expired() {
    let mut map = load_all_pending();
    let mut changed = false;

    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        let should_remove = if let Some(comments) = map.get_mut(&key) {
            let before = comments.len();
            comments.retain(|c| !is_expired(&c.stored_at));
            if comments.len() != before {
                changed = true;
            }
            comments.is_empty()
        } else {
            false
        };
        if should_remove {
            map.remove(&key);
            changed = true;
        }
    }

    if changed {
        if let Ok(json) = serde_json::to_string(&map) {
            write_storage(PENDING_KEY, &json);
        }
    }
}

/// 加载全部待发布评论草稿映射。
fn load_all_pending() -> PendingMap {
    let json = match read_storage(PENDING_KEY) {
        Some(j) => j,
        None => return PendingMap::new(),
    };
    serde_json::from_str(&json).unwrap_or_default()
}

/// 相对时间分档：根据"距现在的毫秒数"返回 (相对文本, 绝对日期 YYYY-MM-DD)。
///
/// 分档规则与服务端 `format_relative_time` 完全一致，前端在展示待审核评论时复用，
/// 保证两类评论的时间展示口径统一。返回绝对日期用于 `title` 悬浮提示。
///
/// - `delta_millis`：目标时间与"现在"的差值（毫秒）。正值表示过去，负值表示未来（兜底按刚刚处理）。
/// - `created_iso`：评论的 RFC3339 创建时间，用于兜底生成绝对日期。
pub fn relative_label_from_millis(delta_millis: i64, created_iso: &str) -> (String, String) {
    let seconds = delta_millis / 1000;

    let label = if seconds < 60 {
        "刚刚".to_string()
    } else {
        let minutes = seconds / 60;
        if minutes < 60 {
            format!("{} 分钟前", minutes)
        } else {
            let hours = minutes / 60;
            if hours < 24 {
                format!("{} 小时前", hours)
            } else {
                let days = hours / 24;
                if days < 30 {
                    format!("{} 天前", days)
                } else {
                    // 超过 30 天直接显示日期，下方 absolute 复用
                    String::new()
                }
            }
        }
    };

    // 绝对日期：优先解析 ISO；解析失败时退化为空串，避免组件报错。
    let absolute = DateTime::parse_from_rfc3339(created_iso)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default();

    let label = if label.is_empty() {
        absolute.clone()
    } else {
        label
    };
    (label, absolute)
}

/// 前端友好的相对时间格式化：返回相对文本，用于展示待审核评论的创建时间。
///
/// 这是 `relative_label_from_millis` 的薄封装，仅返回相对文本。
pub fn format_relative_time_iso(created_iso: &str) -> String {
    // 解析失败时退化为 "刚刚"，避免组件崩溃。
    let Ok(dt) = DateTime::parse_from_rfc3339(created_iso) else {
        return "刚刚".to_string();
    };
    let delta_millis = now_millis() - dt.timestamp_millis();
    relative_label_from_millis(delta_millis, created_iso).0
}

/// HTML 转义辅助函数。
pub(crate) fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// 将待发布评论的 Markdown 内容渲染为安全的 HTML（纯文本 + 换行转 `<br>`）。
pub fn render_pending_content(md: &str) -> String {
    let escaped = escape_html(md);
    escaped.replace('\n', "<br>")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISO: &str = "2026-06-22T05:43:57.565+00:00";

    #[test]
    fn relative_label_just_now_under_60s() {
        let (label, _) = relative_label_from_millis(0, ISO);
        assert_eq!(label, "刚刚");
        let (label, _) = relative_label_from_millis(59_999, ISO);
        assert_eq!(label, "刚刚");
    }

    #[test]
    fn relative_label_minutes() {
        let (label, _) = relative_label_from_millis(60_000, ISO);
        assert_eq!(label, "1 分钟前");
        let (label, _) = relative_label_from_millis(5 * 60_000, ISO);
        assert_eq!(label, "5 分钟前");
        let (label, _) = relative_label_from_millis(59 * 60_000, ISO);
        assert_eq!(label, "59 分钟前");
    }

    #[test]
    fn relative_label_hours() {
        let (label, _) = relative_label_from_millis(60 * 60_000, ISO);
        assert_eq!(label, "1 小时前");
        let (label, _) = relative_label_from_millis(3 * 3600_000, ISO);
        assert_eq!(label, "3 小时前");
        let (label, _) = relative_label_from_millis(23 * 3600_000, ISO);
        assert_eq!(label, "23 小时前");
    }

    #[test]
    fn relative_label_days() {
        let (label, _) = relative_label_from_millis(24 * 3600_000, ISO);
        assert_eq!(label, "1 天前");
        let (label, _) = relative_label_from_millis(7 * 24 * 3600_000, ISO);
        assert_eq!(label, "7 天前");
        let (label, _) = relative_label_from_millis(29 * 24 * 3600_000, ISO);
        assert_eq!(label, "29 天前");
    }

    #[test]
    fn relative_label_falls_back_to_date_over_30_days() {
        let (label, absolute) = relative_label_from_millis(60 * 24 * 3600_000, ISO);
        assert_eq!(label, "2026-06-22");
        assert_eq!(absolute, "2026-06-22");
    }

    #[test]
    fn relative_label_future_falls_back_to_just_now() {
        // 未来时间差为负，秒数 < 60，归为"刚刚"。
        let (label, _) = relative_label_from_millis(-5_000, ISO);
        assert_eq!(label, "刚刚");
    }

    #[test]
    fn relative_label_invalid_iso_still_returns_absolute_empty() {
        // 无法解析时 absolute 为空，但分档逻辑仍按 delta 决定。
        let (label, absolute) = relative_label_from_millis(0, "not-a-date");
        assert_eq!(label, "刚刚");
        assert_eq!(absolute, "");
    }

    #[test]
    fn format_relative_time_iso_invalid_iso_falls_back() {
        // 解析失败退化为"刚刚"，不 panic。
        assert_eq!(format_relative_time_iso("not-a-date"), "刚刚");
    }
}

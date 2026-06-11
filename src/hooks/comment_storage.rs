use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const AUTHOR_KEY: &str = "yggdrasil-comment-author";
const PENDING_KEY: &str = "yggdrasil-pending-comments";
const TTL_DAYS: i64 = 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingComment {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub depth: i32,
    pub author_name: String,
    pub author_url: Option<String>,
    pub avatar_url: String,
    pub content_md: String,
    pub created_at: String,
    pub stored_at: String,
}

type PendingMap = std::collections::HashMap<String, Vec<PendingComment>>;

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

fn is_expired(stored_at: &str) -> bool {
    let Ok(dt) = DateTime::parse_from_rfc3339(stored_at) else {
        return true;
    };
    let now_ms = now_millis();
    let stored_ms = dt.timestamp_millis();
    (now_ms - stored_ms) > (TTL_DAYS * 24 * 60 * 60 * 1000)
}

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

pub fn load_author() -> Option<AuthorInfo> {
    let json = read_storage(AUTHOR_KEY)?;
    serde_json::from_str(&json).ok()
}

pub fn save_pending_comment(post_id: i32, comment: PendingComment) {
    let mut map: PendingMap = load_all_pending();
    let key = post_id.to_string();
    let list = map.entry(key).or_default();

    if list.iter().any(|c| c.id == comment.id) {
        return;
    }
    list.push(comment);

    if let Ok(json) = serde_json::to_string(&map) {
        write_storage(PENDING_KEY, &json);
    }
}

pub fn load_pending_comments(post_id: i32) -> Vec<PendingComment> {
    let mut map = load_all_pending();
    let key = post_id.to_string();

    let comments = map.remove(&key).unwrap_or_default();
    let original_len = comments.len();
    let non_expired: Vec<PendingComment> = comments
        .into_iter()
        .filter(|c| !is_expired(&c.stored_at))
        .collect();

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

fn load_all_pending() -> PendingMap {
    let json = match read_storage(PENDING_KEY) {
        Some(j) => j,
        None => return PendingMap::new(),
    };
    serde_json::from_str(&json).unwrap_or_default()
}

pub(crate) fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn render_pending_content(md: &str) -> String {
    let escaped = escape_html(md);
    escaped.replace('\n', "<br>")
}

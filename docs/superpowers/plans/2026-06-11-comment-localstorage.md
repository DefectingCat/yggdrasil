# Comment localStorage Persistence + Pending Visibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Auto-fill comment form from localStorage and show pending comments with "审核中" badge to the submitting user.

**Architecture:** Client-side localStorage stores author info and pending comment data keyed by post_id. A new server function `check_pending_status` validates local pending IDs on page load. Pending comments render via a separate `PendingCommentItem` component merged chronologically with approved comments in `CommentList`.

**Tech Stack:** Dioxus 0.7, `web_sys` for localStorage, `serde_json` for serialization, `chrono` for timestamps.

---

## Task 1: Add `comment_id` to `CommentResponse` + extract ID in `create.rs`

**Files:**
- Modify: `src/api/comments/types.rs:16-20`
- Modify: `src/api/comments/create.rs:190-221`

- [ ] **Step 1: Add `comment_id` field to `CommentResponse`**

In `src/api/comments/types.rs`, change the struct to:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentResponse {
    pub success: bool,
    pub message: String,
    pub error_code: Option<String>,
    #[serde(default)]
    pub comment_id: Option<i64>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub depth: Option<i32>,
}
```

- [ ] **Step 2: Add `comment_id: None` to all error-path `CommentResponse` constructors in `create.rs`**

Every early-return `Ok(CommentResponse { ... })` in `create.rs` needs `comment_id: None, avatar_url: None, depth: None,` added. There are 12 such sites (lines 27, 36, 43, 50, 58, 78, 88, 109, 121, 128, 137, 164). Add these three fields after `error_code: ...` in each.

- [ ] **Step 3: Extract returned ID and set it in the success response**

Replace `create.rs:190-221` with:

```rust
        let row = client
            .query_one(
                "INSERT INTO comments \
                 (post_id, parent_id, depth, author_name, author_email, author_url, \
                  content_md, content_html, content_hash, status, ip_address, user_agent) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', $10, $11) \
                 RETURNING id",
                &[
                    &post_id,
                    &parent_id,
                    &depth,
                    &author_name.trim(),
                    &author_email.trim(),
                    &author_url.as_ref().map(|u| u.trim()).filter(|u| !u.is_empty()),
                    &content_md,
                    &content_html,
                    &content_hash,
                    &ip_address,
                    &user_agent,
                ],
            )
            .await
            .map_err(AppError::query)?;

        let comment_id: i64 = row.get(0);

        let avatar_url = crate::api::comments::helpers::gravatar_url(&author_email);

        cache::invalidate_comments_by_post(post_id).await;
        cache::invalidate_comment_count(post_id).await;

        Ok(CommentResponse {
            success: true,
            message: "评论已提交，等待审核".to_string(),
            error_code: None,
            comment_id: Some(comment_id),
            avatar_url: Some(avatar_url),
            depth: Some(depth),
        })
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check 2>&1 | head -30`
Expected: no errors related to `CommentResponse`.

- [ ] **Step 5: Commit**

```bash
git add src/api/comments/types.rs src/api/comments/create.rs
git commit -m "feat(api): return comment_id, avatar_url, depth from create_comment

- Add comment_id/avatar_url/depth Option fields to CommentResponse with serde default
- Extract RETURNING id from INSERT in create.rs
- Compute gravatar_url server-side (md5 not available in WASM)
- Return computed depth for correct client-side pending comment indentation
- All error paths return None for all new fields"
```

---

## Task 2: Create `CheckPendingStatus` server function

**Files:**
- Create: `src/api/comments/check.rs`
- Modify: `src/api/comments/mod.rs:1-17`

- [ ] **Step 1: Create `src/api/comments/check.rs`**

```rust
use dioxus::prelude::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingStatusItem {
    pub id: i64,
    pub status: String,
}

#[server(CheckPendingStatus, "/api")]
pub async fn check_pending_status(ids: Vec<i64>) -> Result<Vec<PendingStatusItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::db::pool::get_conn;
        use crate::api::error::AppError;

        if ids.is_empty() {
            return Ok(vec![]);
        }

        let client = get_conn().await.map_err(AppError::db_conn)?;

        let rows = client
            .query(
                "SELECT id, status FROM comments WHERE id = ANY($1)",
                &[&ids],
            )
            .await
            .map_err(AppError::query)?;

        let found: std::collections::HashMap<i64, String> = rows
            .iter()
            .map(|r| (r.get::<_, i64>(0), r.get::<_, String>(1)))
            .collect();

        let result: Vec<PendingStatusItem> = ids
            .into_iter()
            .map(|id| {
                let status = found.get(&id).cloned().unwrap_or_else(|| "gone".to_string());
                PendingStatusItem { id, status }
            })
            .collect();

        Ok(result)
    }
    #[cfg(not(feature = "server"))]
    unreachable!()
}
```

- [ ] **Step 2: Register module and export in `src/api/comments/mod.rs`**

Add `mod check;` to the module declarations and `pub use check::check_pending_status;` to the exports:

```rust
#![allow(clippy::unused_unit, deprecated, unused_imports, clippy::too_many_arguments)]

mod types;
mod helpers;
mod markdown;
mod create;
mod read;
mod update;
mod list;
mod check;

pub use types::*;
pub use create::create_comment;
pub use read::{get_comments, get_comment_count};
pub use update::{approve_comment, spam_comment, trash_comment, batch_update_comment_status};
pub use list::{get_pending_comments, get_pending_count, get_all_comments};
pub use check::check_pending_status;

#[cfg(feature = "server")]
pub use markdown::render_comment_markdown;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check 2>&1 | head -30`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/api/comments/check.rs src/api/comments/mod.rs
git commit -m "feat(api): add CheckPendingStatus server function

Accepts Vec<i64> of comment IDs, returns their current status.
IDs not found in DB return status 'gone'. Empty vec returns early.
Used by client to prune localStorage pending comments that are
no longer pending (approved/spam/trash/deleted)."
```

---

## Task 3: Create `comment_storage` hook

**Files:**
- Create: `src/hooks/comment_storage.rs`
- Modify: `src/hooks/mod.rs:1`

- [ ] **Step 1: Create `src/hooks/comment_storage.rs`**

```rust
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn is_expired(stored_at: &str) -> bool {
    let Ok(dt) = DateTime::parse_from_rfc3339(stored_at) else {
        return true;
    };
    let Ok(now) = DateTime::parse_from_rfc3339(&now_iso()) else {
        return false;
    };
    (now - dt).num_days() > TTL_DAYS
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
    let non_expired: Vec<PendingComment> = comments
        .into_iter()
        .filter(|c| !is_expired(&c.stored_at))
        .collect();

    if !non_expired.is_empty() {
        map.insert(key, non_expired.clone());
    }
    if let Ok(json) = serde_json::to_string(&map) {
        write_storage(PENDING_KEY, &json);
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

pub fn escape_html(input: &str) -> String {
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
```

- [ ] **Step 2: Register module in `src/hooks/mod.rs`**

```rust
pub mod delayed_loading;
pub mod comment_storage;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check 2>&1 | head -30`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/hooks/comment_storage.rs src/hooks/mod.rs
git commit -m "feat(hooks): add comment_storage module for localStorage persistence

Provides save/load for author info (yggdrasil-comment-author) and
pending comments (yggdrasil-pending-comments) in localStorage.
- 7-day TTL with auto-pruning
- Per-post-id storage keyed by post_id string
- HTML escaping for pending content_md rendering
- All web_sys calls behind #[cfg(target_arch = \"wasm32\")]"
```

---

## Task 4: Update `CommentContext` and `CommentSection` for pending comments

**Files:**
- Modify: `src/components/comments/section.rs` (full rewrite)

- [ ] **Step 1: Rewrite `src/components/comments/section.rs`**

```rust
use dioxus::prelude::*;

use crate::api::comments::{get_comments, check_pending_status, CommentTreeResponse};
use crate::hooks::comment_storage::{self, PendingComment};
use crate::components::comments::form::CommentForm;
use crate::components::comments::list::CommentList;
use crate::components::skeletons::comment_skeleton::CommentListSkeleton;

#[derive(Clone, Copy)]
pub struct CommentContext {
    pub active_reply: Signal<Option<i64>>,
    pub refresh_trigger: Signal<bool>,
    pub pending_comments: Signal<Vec<PendingComment>>,
}

#[component]
pub fn CommentSection(post_id: i32) -> Element {
    let ctx = use_context_provider(|| {
        let pending: Vec<PendingComment> = comment_storage::load_pending_comments(post_id);
        comment_storage::prune_all_expired();

        CommentContext {
            active_reply: Signal::new(None),
            refresh_trigger: Signal::new(false),
            pending_comments: Signal::new(pending),
        }
    });

    use_future(move || {
        let pending = ctx.pending_comments;
        async move {
            let ids: Vec<i64> = pending().iter().map(|c| c.id).collect();
            if ids.is_empty() {
                return;
            }
            if let Ok(statuses) = check_pending_status(ids).await {
                let to_remove: Vec<i64> = statuses
                    .into_iter()
                    .filter(|s| s.status != "pending")
                    .map(|s| s.id)
                    .collect();
                if !to_remove.is_empty() {
                    comment_storage::remove_pending_ids(post_id, &to_remove);
                    pending.write().retain(|c| !to_remove.contains(&c.id));
                }
            }
        }
    });

    let comments_resource = use_server_future(move || {
        let _ = ctx.refresh_trigger;
        get_comments(post_id)
    })?;

    let data = comments_resource.read();

    match data.as_ref().map(|r| r.as_ref()) {
        Some(Ok(CommentTreeResponse { comments, count })) => {
            let approved_count = *count;
            let pending_count = ctx.pending_comments.read().len() as i64;
            let total_count = approved_count + pending_count;
            let has_any = approved_count > 0 || pending_count > 0;
            rsx! {
                div { class: "space-y-8",
                    h2 { class: "text-xl font-bold text-paper-primary",
                        "评论区 ({total_count})"
                    }

                    CommentForm { post_id, parent_id: None }

                    if !has_any {
                        p { class: "text-paper-tertiary text-center py-8",
                            "暂无评论，成为第一个评论的人吧！"
                        }
                    } else {
                        CommentList {
                            comments: comments.clone(),
                            pending: ctx.pending_comments.read().clone(),
                            post_id,
                        }
                    }
                }
            }
        }
        Some(Err(_)) => {
            rsx! {
                div { class: "text-center text-red-500 dark:text-red-400 py-8",
                    "评论加载失败"
                }
            }
        }
        None => rsx! { CommentListSkeleton {} },
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | head -30`
Expected: may show errors in `CommentList` (signature changed) — that's expected, fixed in Task 5.

- [ ] **Step 3: Commit**

```bash
git add src/components/comments/section.rs
git commit -m "feat(comments): add pending_comments to CommentContext and sync on mount

- Extend CommentContext with pending_comments Signal
- Load pending comments from localStorage on provider init
- Run check_pending_status on mount to prune non-pending entries
- Pass both approved and pending comments to CommentList
- Include pending count in section heading"
```

---

## Task 5: Update `CommentList` to merge approved + pending comments

**Files:**
- Modify: `src/components/comments/list.rs` (full rewrite)

- [ ] **Step 1: Rewrite `src/components/comments/list.rs`**

```rust
use dioxus::prelude::*;

use crate::models::comment::PublicComment;
use crate::hooks::comment_storage::PendingComment;
use crate::components::comments::item::CommentItem;
use crate::components::comments::pending_item::PendingCommentItem;

enum MergedComment {
    Approved(PublicComment),
    Pending(PendingComment),
}

fn merge_comments(
    approved: Vec<PublicComment>,
    pending: Vec<PendingComment>,
) -> Vec<MergedComment> {
    let mut merged: Vec<MergedComment> = approved
        .into_iter()
        .map(MergedComment::Approved)
        .chain(pending.into_iter().map(MergedComment::Pending))
        .collect();

    merged.sort_by(|a, b| {
        let time_a = match a {
            MergedComment::Approved(c) => c.created_at_iso.as_str(),
            MergedComment::Pending(c) => c.created_at.as_str(),
        };
        let time_b = match b {
            MergedComment::Approved(c) => c.created_at_iso.as_str(),
            MergedComment::Pending(c) => c.created_at.as_str(),
        };
        time_a.cmp(time_b)
    });

    merged
}

#[component]
pub fn CommentList(
    comments: Vec<PublicComment>,
    pending: Vec<PendingComment>,
    post_id: i32,
) -> Element {
    let merged = merge_comments(comments, pending);

    rsx! {
        div { class: "space-y-0 divide-y divide-gray-100 dark:divide-[#2a2a2a]",
            for item in merged {
                match item {
                    MergedComment::Approved(comment) => rsx! {
                        CommentItem { comment, post_id }
                    },
                    MergedComment::Pending(comment) => rsx! {
                        PendingCommentItem { comment, post_id }
                    },
                }
            }
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | head -30`
Expected: error about missing `pending_item` module — that's expected, fixed in Task 6.

- [ ] **Step 3: Commit**

```bash
git add src/components/comments/list.rs
git commit -m "feat(comments): merge approved and pending comments in CommentList

- Accept both comments and pending props
- Merge into chronologically sorted list
- Route to CommentItem or PendingCommentItem per item type"
```

---

## Task 6: Create `PendingCommentItem` component

**Files:**
- Create: `src/components/comments/pending_item.rs`
- Modify: `src/components/comments/mod.rs:1-5`

- [ ] **Step 1: Create `src/components/comments/pending_item.rs`**

```rust
use dioxus::prelude::*;

use crate::hooks::comment_storage::{PendingComment, render_pending_content};

#[component]
pub fn PendingCommentItem(comment: PendingComment, post_id: i32) -> Element {
    let _ = post_id;

    let depth = if comment.parent_id.is_none() && comment.depth > 0 {
        0
    } else {
        comment.depth
    };

    let indent = depth.min(6) * 24;
    let content_html = render_pending_content(&comment.content_md);

    let author_element = match &comment.author_url {
        Some(url) if !url.is_empty() => rsx! {
            a {
                href: "{url}",
                rel: "nofollow noopener",
                target: "_blank",
                class: "font-medium text-paper-primary hover:text-paper-accent transition-colors",
                "{comment.author_name}"
            }
        },
        _ => rsx! {
            span { class: "font-medium text-paper-primary",
                "{comment.author_name}"
            }
        },
    };

    rsx! {
        div {
            class: "py-4 opacity-70",
            style: "margin-left: {indent}px",

            div { class: "flex gap-3",
                img {
                    src: "{comment.avatar_url}",
                    alt: "{comment.author_name} 的头像",
                    loading: "lazy",
                    decoding: "async",
                    class: "w-8 h-8 rounded-full shrink-0 mt-0.5 bg-gray-200 dark:bg-[#2a2a2a]",
                }

                div { class: "flex-1 min-w-0",
                    div { class: "flex items-center gap-1.5 text-sm mb-1.5 flex-wrap",
                        {author_element}
                        span { class: "text-paper-tertiary", "·" }
                        span {
                            class: "text-paper-tertiary",
                            "刚刚"
                        }
                        span {
                            class: "inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400",
                            "审核中"
                        }
                    }

                    div {
                        class: "prose prose-sm dark:prose-invert max-w-none text-paper-secondary",
                        dangerous_inner_html: "{content_html}",
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Register module in `src/components/comments/mod.rs`**

```rust
pub mod section;
pub mod form;
pub mod list;
pub mod item;
pub mod pending_item;
pub mod actions;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check 2>&1 | head -30`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/comments/pending_item.rs src/components/comments/mod.rs
git commit -m "feat(comments): add PendingCommentItem component

Renders pending (unapproved) comments with:
- opacity-70 for visual distinction
- amber '审核中' badge
- Client-side content_md rendering (HTML escape + newline→br)
- No reply button (server rejects replies to pending parents)
- Same depth/indent logic as approved comments"
```

---

## Task 7: Update `CommentForm` for auto-fill + localStorage save

**Files:**
- Modify: `src/components/comments/form.rs` (full rewrite)

- [ ] **Step 1: Rewrite `src/components/comments/form.rs`**

```rust
use dioxus::prelude::*;

use crate::api::comments::create_comment;
use crate::components::comments::section::CommentContext;
use crate::components::forms::{INPUT_CLASS, BUTTON_PRIMARY_CLASS, AlertBox};
use crate::hooks::comment_storage::{self, PendingComment};

#[component]
pub fn CommentForm(post_id: i32, parent_id: Option<i64>) -> Element {
    let ctx: CommentContext = use_context();
    let mut active_reply = ctx.active_reply;
    let mut refresh_trigger = ctx.refresh_trigger;
    let mut pending_comments = ctx.pending_comments;

    let mut author_name = use_signal(String::new);
    let mut author_email = use_signal(String::new);
    let mut author_url = use_signal(String::new);
    let mut content_md = use_signal(String::new);
    let mut honeypot = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    let mut message = use_signal(|| Option::<(String, &'static str)>::None);

    use_effect(move || {
        if !author_name().is_empty() {
            return;
        }
        if let Some(info) = comment_storage::load_author() {
            author_name.set(info.name);
            author_email.set(info.email);
            author_url.set(info.url);
        }
    });

    if let Some(pid) = parent_id {
        if active_reply() != Some(pid) {
            return rsx! {};
        }
    }

    let is_reply = parent_id.is_some();

    rsx! {
        div {
            class: if is_reply { "mt-3 pt-3 border-t border-gray-100 dark:border-[#333]" } else { "" },
            role: "form",
            aria_label: if is_reply { "回复评论" } else { "发表评论" },

            if let Some((msg, variant)) = message() {
                div { aria_live: "polite",
                    AlertBox { message: msg, variant }
                }
            }

            div { class: "space-y-3",
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3",
                    div {
                        label { class: "block text-sm font-medium text-paper-secondary mb-1",
                            "昵称 *"
                        }
                        input {
                            class: INPUT_CLASS,
                            r#type: "text",
                            placeholder: "你的昵称",
                            value: "{author_name}",
                            disabled: submitting(),
                            oninput: move |e| author_name.set(e.value()),
                        }
                    }
                    div {
                        label { class: "block text-sm font-medium text-paper-secondary mb-1",
                            "邮箱 *"
                        }
                        input {
                            class: INPUT_CLASS,
                            r#type: "email",
                            placeholder: "your@email.com",
                            value: "{author_email}",
                            disabled: submitting(),
                            oninput: move |e| author_email.set(e.value()),
                        }
                    }
                }
                div {
                    label { class: "block text-sm font-medium text-paper-secondary mb-1",
                        "网站"
                    }
                    input {
                        class: INPUT_CLASS,
                        r#type: "url",
                        placeholder: "https://example.com（可选）",
                        value: "{author_url}",
                        disabled: submitting(),
                        oninput: move |e| author_url.set(e.value()),
                    }
                }

                textarea {
                    class: "{INPUT_CLASS} min-h-[100px] resize-y",
                    value: "{content_md}",
                    disabled: submitting(),
                    oninput: move |e| content_md.set(e.value()),
                }

                p { class: "text-xs text-paper-tertiary",
                    "支持 Markdown 语法"
                }

                textarea {
                    class: "hidden",
                    aria_hidden: "true",
                    tabindex: "-1",
                    value: "{honeypot}",
                    oninput: move |e| honeypot.set(e.value()),
                }

                button {
                    class: BUTTON_PRIMARY_CLASS,
                    disabled: submitting(),
                    onclick: move |_| {
                        let post_id = post_id;
                        let parent_id = parent_id;
                        let name = author_name();
                        let email = author_email();
                        let url_val = author_url();
                        let content = content_md();
                        let hp = honeypot();

                        if !hp.is_empty() {
                            return;
                        }

                        if name.trim().is_empty() || email.trim().is_empty() || content.trim().is_empty() {
                            message.set(Some(("请填写所有必填项".to_string(), "error")));
                            return;
                        }

                        submitting.set(true);
                        message.set(None);

                        spawn(async move {
                            let result = create_comment(
                                post_id,
                                parent_id,
                                name.clone(),
                                email.clone(),
                                if url_val.trim().is_empty() { None } else { Some(url_val.clone()) },
                                content.clone(),
                            ).await;

                            submitting.set(false);

                            match result {
                                Ok(resp) => {
                                    if resp.success {
                                        comment_storage::save_author(
                                            &name,
                                            &email,
                                            &url_val,
                                        );

                                        if let Some(comment_id) = resp.comment_id {
                                            let avatar_url = resp.avatar_url.unwrap_or_default();
                                            let depth = resp.depth.unwrap_or(0);

                                            let now = chrono::Utc::now().to_rfc3339();
                                            let pending = PendingComment {
                                                id: comment_id,
                                                parent_id,
                                                depth,
                                                author_name: name.clone(),
                                                author_url: if url_val.trim().is_empty() { None } else { Some(url_val) },
                                                avatar_url,
                                                content_md: content,
                                                created_at: now.clone(),
                                                stored_at: now,
                                            };

                                            comment_storage::save_pending_comment(post_id, pending.clone());
                                            pending_comments.write().push(pending);
                                        }

                                        content_md.set(String::new());
                                        message.set(Some((resp.message, "success")));
                                        if parent_id.is_some() {
                                            active_reply.set(None);
                                        }
                                        refresh_trigger.set(!refresh_trigger());
                                    } else {
                                        message.set(Some((resp.message, "error")));
                                    }
                                }
                                Err(_) => {
                                    message.set(Some(("提交失败，请稍后重试".to_string(), "error")));
                                }
                            }
                        });
                    },

                    if submitting() {
                        "提交中…"
                    } else if is_reply {
                        "回复"
                    } else {
                        "发表评论"
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | head -30`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/comments/form.rs
git commit -m "feat(comments): auto-fill form from localStorage and save pending comments

- Load author info from localStorage on mount via use_effect
- Save author info + pending comment to localStorage on successful submit
- Use server-returned avatar_url and depth (no client-side md5 needed)
- Push pending comment to CommentContext signal for immediate render
- Pre-fill works for both main form and reply forms"
```

---

## Task 8: Run full build + tests

- [ ] **Step 1: Run cargo test**

Run: `cargo test 2>&1 | tail -20`
Expected: all tests pass.

- [ ] **Step 2: Run cargo clippy**

Run: `cargo clippy 2>&1 | tail -20`
Expected: no warnings on changed files.

- [ ] **Step 3: Run cargo check**

Run: `cargo check 2>&1 | tail -10`
Expected: no errors.

- [ ] **Step 4: Final commit if any fixups needed**

```bash
git add -A
git commit -m "chore: fix compilation/lint issues from comment localStorage feature"
```

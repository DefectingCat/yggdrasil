# Comment localStorage Persistence + Pending Visibility

## Problem

After submitting a comment, users see only a green "评论已提交，等待审核" alert. The comment is invisible to everyone (including the author) until admin approval. Additionally, returning users must re-enter their name/email/website every time.

## Solution

Two localStorage-backed features:

1. **Form auto-fill**: Save author info (name/email/url) to localStorage, pre-fill on subsequent visits.
2. **Pending comment visibility**: Store pending comments locally by server-returned ID. Display them with a "审核中" badge, visible only to the submitting browser.

## localStorage Keys

### `yggdrasil-comment-author`

Written on every successful comment submission. Read on `CommentForm` mount.

```json
{ "name": "张三", "email": "zhang@example.com", "url": "https://example.com" }
```

### `yggdrasil-pending-comments`

Written on successful submission with server-returned ID. Read on `CommentSection` mount. Keyed by `post_id` (string).

```json
{
  "42": [
    {
      "id": 123,
      "parent_id": null,
      "depth": 0,
      "author_name": "张三",
      "author_url": null,
      "avatar_url": "https://cravatar.cn/avatar/xxx?d=mp&s=80",
      "content_md": "评论内容",
      "created_at": "2026-06-11T10:00:00Z",
      "stored_at": "2026-06-11T10:00:00Z"
    }
  ]
}
```

Design decisions:
- **No `content_html`** — XSS safety. Render `content_md` client-side with HTML escaping + newline→`<br>`.
- **No `author_email`** — Already in `yggdrasil-comment-author`. Avoid PII duplication.
- **`avatar_url`** computed at save time from the stored author email.
- **`stored_at`** for 7-day TTL. Pruned on every read.
- Empty post_id arrays removed on cleanup.

## Server-Side Changes

### 1. `CommentResponse` — add `comment_id`

```rust
pub struct CommentResponse {
    pub success: bool,
    pub message: String,
    pub error_code: Option<String>,
    pub comment_id: Option<i64>,  // NEW: set only on success
}
```

Backward-compatible: `Option<i64>` serde defaults to `None` for old callers.

### 2. `create_comment` — extract and return ID

The existing INSERT already uses `RETURNING id` but discards the row. Change to:

```rust
let row = client
    .query_one("INSERT INTO comments ... RETURNING id", &[...])
    .await
    .map_err(AppError::query)?;
let comment_id: i64 = row.get(0);
```

All existing error paths add `comment_id: None`. Only the final success path sets `comment_id: Some(comment_id)`.

### 3. New server function: `CheckPendingStatus`

```rust
#[server(CheckPendingStatus, "/api")]
pub async fn check_pending_status(ids: Vec<i64>) -> Result<Vec<(i64, String)>, ServerFnError>
```

- Early return empty vec if `ids.is_empty()`.
- Query: `SELECT id, status FROM comments WHERE id = ANY($1)`
- IDs not found in result → status `"gone"` (soft-deleted or hard-deleted).
- Client uses this to prune localStorage entries that are no longer pending.

## Client-Side Changes

### New module: `src/hooks/comment_storage.rs`

Provides `use_comment_storage()` hook with:

| Function | Purpose |
|----------|---------|
| `save_author(name, email, url)` | Write `yggdrasil-comment-author` |
| `load_author() -> Option<AuthorInfo>` | Read author info |
| `save_pending_comment(post_id, comment)` | Append to `yggdrasil-pending-comments` |
| `load_pending_comments(post_id) -> Vec<PendingComment>` | Read + prune expired (7-day TTL) |
| `remove_pending_ids(post_id, ids)` | Remove specific IDs, clean empty post entries |
| `prune_expired()` | Remove all entries across all posts older than 7 days |

All `web_sys` calls behind `#[cfg(target_arch = "wasm32")]`. Non-WASM returns defaults (empty/None).

Serialization via `serde_json` (already a project dependency).

### `CommentContext` — extend with pending state

```rust
#[derive(Clone, Copy)]
pub struct CommentContext {
    pub active_reply: Signal<Option<i64>>,
    pub refresh_trigger: Signal<bool>>,
    pub pending_comments: Signal<Vec<PendingComment>>,  // NEW
}
```

Placing `pending_comments` in `CommentContext` ensures it survives `refresh_trigger` re-renders (blocker B2 from review).

### `CommentForm` changes

- **On mount**: call `load_author()` → set `author_name`, `author_email`, `author_url` signals if localStorage has saved values.
- **On successful submit**:
  1. `save_author(name, email, url)` — persist form fields
  2. Construct `PendingComment` from form data + returned `comment_id`
  3. Compute `avatar_url` from stored email (same gravatar formula as server)
  4. `save_pending_comment(post_id, pending)` — persist pending comment
  5. Push to `ctx.pending_comments` signal — immediate UI update

### `CommentSection` changes

- **On mount**:
  1. `load_pending_comments(post_id)` → populate `ctx.pending_comments`
  2. `prune_expired()` — clean 7-day old entries
  3. If pending IDs exist, call `check_pending_status(ids)` → `remove_pending_ids()` for non-pending
- **Rendering**: merge approved + pending comments sorted by `created_at`, interleaving them chronologically.

### New component: `PendingCommentItem`

Renders pending comments distinctly from approved ones:
- Semi-transparent card style (e.g., `opacity-70`)
- Amber badge: "审核中"
- `content_md` rendered with HTML escaping + `\n` → `<br>`
- **No reply button** — server rejects replies to non-approved parents
- **No admin actions** — pending comments from localStorage are visitor-submitted

### `CommentList` changes

Accept both `comments: Vec<PublicComment>` and `pending: Vec<PendingComment>`. Merge into a sorted iterator by `created_at` and render either `CommentItem` or `PendingCommentItem` per item.

For threaded display: pending replies appear under their `parent_id` parent, using the same `depth` indentation logic.

## Data Flow

```
Submit comment
  → server returns { success: true, comment_id: 123 }
  → save_author() to localStorage
  → save_pending_comment() to localStorage
  → push to pending_comments Signal
  → UI shows comment with "审核中" badge

Page load / CommentSection mount
  → load_pending_comments(post_id) from localStorage
  → prune_expired() (7-day TTL)
  → check_pending_status(pending_ids) via server
  → remove_pending_ids() for non-pending (approved/spam/trash/gone)
  → merge approved + pending → render chronologically

Admin approves comment
  → next page load: check_pending_status() returns "approved"
  → remove_pending_ids() removes it from localStorage
  → comment appears normally via get_comments() API
```

## Files Changed

| File | Change |
|------|--------|
| `src/api/comments/types.rs` | Add `comment_id: Option<i64>` to `CommentResponse` |
| `src/api/comments/create.rs` | Extract returned ID, populate `comment_id` in response |
| `src/api/comments/mod.rs` | Export new `check_pending_status` |
| `src/api/comments/check.rs` | **NEW**: `CheckPendingStatus` server function |
| `src/hooks/comment_storage.rs` | **NEW**: localStorage hook + `AuthorInfo`/`PendingComment` structs |
| `src/hooks/mod.rs` | Export `comment_storage` module |
| `src/components/comments/section.rs` | Load pending, call check, merge for rendering |
| `src/components/comments/form.rs` | Pre-fill from localStorage, save on submit |
| `src/components/comments/list.rs` | Accept both comment types, merge sorted |
| `src/components/comments/item.rs` | No change (approved comments unchanged) |
| `src/components/comments/pending_item.rs` | **NEW**: `PendingCommentItem` component |
| `src/components/comments/mod.rs` | Export `pending_item` |

## Testing

- Unit test: `CommentResponse` deserialization without `comment_id` → `None`
- Unit test: `PendingComment` serde roundtrip
- Unit test: `check_pending_status` with empty vec → empty result
- Unit test: `check_pending_status` with mix of pending/approved/gone IDs
- Integration: submit comment → verify localStorage written → verify pending visible → verify cleanup after admin approval

# T4 — write-scope tools (post CRUD, comments, tags, media)

## Blocking edges
- **Blocks:** T6 (hardening covers these).
- **Blocked by:** T1 (mount + auth + dispatch exist).

## Target files
- `src/mcp/tools/posts.rs`, `src/mcp/tools/comments.rs`, `src/mcp/tools/tags.rs`,
  `src/mcp/tools/media.rs` — new.
- `src/mcp/server.rs` — register write tools (scope-gated).

## Change
Reuse existing server-fn / helper logic — do NOT duplicate business rules. Each write
tool calls the same helpers the web admin uses, then runs the same cache invalidation
(moka `invalidate_*` + `ssr_cache::invalidate_ssr_*`).
1. **Posts:** `create_post`, `update_post`, `publish_post`, `trash_post`, `delete_post`.
   `write`/`admin` scope. These tokens MAY read drafts (decision 7). `create_post` must
   `spawn_blocking(render_markdown_enhanced)` and persist `content_html`, exactly like
   `src/api/posts/create.rs`.
2. **Comments:** `list_comments`, `approve_comment`, `delete_comment`,
   `set_comment_status` — delegate to `src/api/comments/*` helpers.
3. **Tags:** `create_tag`, `rename_tag` — delegate to `src/api/posts/tags.rs`.
4. **Media:** `upload_media(filename, base64, alt?)` — feed the existing upload pipeline
   (`src/api/upload.rs`); return the served URL for embedding in posts.

## Acceptance
- A `write`-token `create_post` writes a row, renders `content_html`, and invalidates the
  matching moka + SSR caches (web admin sees the new post immediately).
- A `read` token calling any write tool → `insufficient_scope` rejection.
- `write`/`admin` tokens can read drafts via `get_post`; `read` tokens cannot.
- Media upload returns a usable image URL consumable in post body.
- Both targets compile; clippy clean; existing tests pass.

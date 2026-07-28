# T3 — read-scope tools + Resources (full knowledge base)

## Blocking edges
- **Blocks:** T6 (hardening sanitizes these outputs).
- **Blocked by:** T1 (mount + auth + dispatch exist).

## Target files
- `src/mcp/resources.rs` — Resources + templates.
- `src/mcp/tools/read.rs` — full read tool set.
- `src/mcp/server.rs` — register resources/templates + read tools in the handler.

## Change
1. `resources/list(cursor?, limit?)` — paginated PUBLISHED posts (opaque cursor;
   page size server-set, e.g. 50). Returns `{resources: [{uri, name, mimeType:
   text/markdown}], nextCursor}`. Invalid cursor → MCP error `-32602`.
2. `resources/templates/list` — `post://{slug}` template.
3. `resources/read(uri)` — resolve `post://{slug}` (or canonical HTTPS URL) → rendered
   Markdown of one published post.
4. `search_posts(query, limit?, cursor?)` — full FTS via existing search helper; return
   ranked summaries + `resource_link`s (content type added 2025-06-18) to matching posts.
5. `get_post(slug|id)` — published only (read scope).
6. `list_tags()`.

## Acceptance
- `resources/list` paginates; `nextCursor` round-trips; invalid cursor errors `-32602`.
- `resources/read` returns one post's Markdown; unknown slug → error.
- `search_posts` returns ranked results with valid `resource_link` URIs.
- All read tools succeed with a `read`-scope token; drafts are NOT visible.
- Pure unit tests: cursor encode/decode, resource URI parsing (DB-free).
- Both targets compile; clippy clean; existing tests pass.

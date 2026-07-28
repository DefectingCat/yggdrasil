# MCP Server — Specification

> Buildable spec for a Model Context Protocol server embedded in Yggdrasil.
> Derives from the grilling decisions (2026-07-28) and `docs/mcp-research.md`.
> Spec version: 1.0.

## 1. Overview

Expose the blog as an **MCP server** so the admin's own AI clients (Claude Code,
Cursor, Cline/Continue) can (a) query published posts as a **knowledge base** and
(b) perform nearly all backend operations (write posts, moderate comments, manage
tags/settings, upload media, run code). An admin settings page generates per-token
MCP client configs with a user-selectable token lifetime and three permission scopes.

**Single-tenant model.** Tokens are minted by the admin for the admin's own clients.
No OAuth 2.1 / PKCE / dynamic client registration. The bearer-token header format is
identical to an OAuth bearer, so the path is forward-compatible if OAuth is ever added.

## 2. Decisions (locked during grilling)

| # | Decision | Choice |
|---|---|---|
| 1 | Audience | Admin's own AI clients only |
| 2 | Scope model | `read` / `write` / `admin` (3 scopes) |
| 3 | Token lifetime | Preset menu: 1d / 7d / 30d / 90d / never-expire (opt-in) |
| 4 | Knowledge-base retrieval | Full-text search MVP (reuse Postgres FTS); published posts as MCP Resources + `search_posts` tool; architecture leaves a seam for future pgvector |
| 5 | Tool surface | Post CRUD, post query, comment mgmt, tag mgmt, site settings, media upload, code runner. **Excludes** DB export/backup |
| 6 | Scope mapping | media=write; code runner=admin-only; read=published only; write/admin=includes drafts |
| 7 | Draft visibility | `read` → published only; `write`/`admin` → incl. drafts |
| 8 | Token storage | AES-GCM encrypted at rest (env key `MCP_TOKEN_ENC_KEY`); retrievable by admin |
| 9 | Config targets | Claude Code, Cursor, Cline/Continue, generic raw JSON |

## 3. Goals / Non-goals

**Goals**
- One `/mcp` Streamable HTTP endpoint, stateless, spec `2025-11-25`.
- Bearer-token auth with 3 scopes, encrypted-at-rest storage, selectable lifetime.
- Knowledge base: published posts as paginated Resources + FTS `search_posts` tool.
- Write/admin tools covering the locked surface.
- Admin UI: token CRUD + one-click config generation for 4 client formats.

**Non-goals (MVP)**
- OAuth 2.1 / PKCE / DCR (forward-compatible seam only).
- Semantic/vector retrieval (pgvector) — full-text first; interface is vector-ready.
- Prompts / Completions / Elicitation / Sampling / Roots / Tasks / Logging primitives.
- Multi-tenant / public token issuance.
- Database export/backup over MCP.

## 4. Architecture

### 4.1 Transport & mount
- **Streamable HTTP, stateless** (SEP-2567): no `Mcp-Session-Id`, no GET stream.
  Each POST is a self-contained request; the bearer token authenticates it.
- Mount the official **`rmcp`** crate's Tower `StreamableHttpService` at `/mcp` via
  `axum::Router::nest_service("/mcp", service)`, merged into the app router in
  `src/main.rs` alongside the existing `upload_route`/`export_route`/`sse_route`.
- Negotiate protocol `2025-11-25`; respond `application/json` (no SSE stream needed
  for non-streaming tools).

### 4.2 Authentication
- `Authorization: Bearer ygg_<opaque>` on every request.
- A new middleware/extractor resolves the bearer → `(user_id, scope)`:
  decrypt-stored-token lookup by token id, enforce expiry, update `last_used_at`.
- **`Origin` → 403 is a hard MUST** (spec `2025-11-25`): validate `Origin` against the
  same trusted-origin allowlist as CSRF (`APP_BASE_URL` / Host fallback).
- `MCP-Protocol-Version` header honored; reject unsupported versions with 400.
- Sessions MUST NOT be used for auth (spec); the token authenticates every request.

### 4.3 Rate limiting
- New **token-keyed** governor bucket on `/mcp` only (extract bearer → `user_id`).
  The existing IP-keyed governor stays on the web app and is NOT reused for MCP.
- Per-request body-size cap; rely on the global `statement_timeout` for DB bounds.

### 4.4 Scope enforcement
- Each tool declares its required scope. Dispatch checks `token.scope >= required`;
  mismatch → `403 insufficient_scope`-style MCP error.
- Read tools (search, get/list published) = `read`. Mutating post/comment/tag/media =
  `write`. Settings + code runner = `admin`.
- Search/snippet output is an **indirect-prompt-injection surface**: sanitize, keep
  read vs write on separate scopes, lean on the client's human-in-the-loop.

### 4.5 Feature gating
- All MCP code is **server-only**: `#[cfg(feature = "server")]` impl + compiling
  `#[cfg(not(feature = "server"))]` stubs for the WASM build, per AGENTS.md §1.
- The admin UI page renders on both targets; it calls token-management server fns.

## 5. Data model

### 5.1 Migration `015_mcp_tokens.sql` (+ register in `MIGRATIONS`)
```sql
CREATE TABLE IF NOT EXISTS mcp_tokens (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    scope         TEXT NOT NULL CHECK (scope IN ('read','write','admin')),
    -- AES-GCM ciphertext of the full bearer token (nonce || ct || tag), hex-encoded.
    token_enc     TEXT NOT NULL,
    -- SHA-256 of the token for fast constant-time lookup by bearer on each request.
    token_hash    CHAR(64) NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at    TIMESTAMPTZ,            -- NULL = never-expire (opt-in)
    last_used_at  TIMESTAMPTZ,
    revoked_at    TIMESTAMPTZ
);
CREATE UNIQUE INDEX mcp_tokens_token_hash_active_idx
    ON mcp_tokens (token_hash) WHERE revoked_at IS NULL;
CREATE INDEX mcp_tokens_user_idx ON mcp_tokens (user_id);
```

### 5.2 Model (`src/models/mcp_token.rs`)
`McpToken` (DB row), `McpTokenSummary` (list response — no secret), `CreateTokenResponse`
(returns plaintext once), plus a `TokenScope` enum (`Read`/`Write`/`Admin`) with
`>=` ordering and serde.

## 6. Module breakdown

New top-level module `src/mcp/` (server-only impl, stubs for WASM):

| Path | Responsibility |
|---|---|
| `src/mcp/mod.rs` | Module root, re-exports, `cfg` gate |
| `src/mcp/crypto.rs` | AES-GCM encrypt/decrypt of tokens; reads `MCP_TOKEN_ENC_KEY` |
| `src/mcp/auth.rs` | Bearer extractor → `(user_id, scope)`; Origin check; `last_used_at` update |
| `src/mcp/rate_limit.rs` | Token-keyed governor for `/mcp` |
| `src/mcp/server.rs` | rmcp `ServerHandler` impl: `tools/list`, `resources/list`, `resources/read`, dispatch |
| `src/mcp/tools/` | Tool impls: `read.rs`, `posts.rs`, `comments.rs`, `tags.rs`, `settings.rs`, `media.rs`, `runner.rs` |
| `src/mcp/resources.rs` | Published-post Resources + templates (`post://{slug}`) |
| `src/mcp/router.rs` | Builds the `StreamableHttpService`, mounts at `/mcp` (called from `main.rs`) |
| `src/mcp/config.rs` | Generates the 4 client config formats from a token |

Token management lives in the existing API tree:
| Path | Responsibility |
|---|---|
| `src/api/mcp_tokens.rs` | `#[server]` fns: `create_mcp_token`, `list_mcp_tokens`, `revoke_mcp_token`, `reveal_mcp_token` (admin-guarded) |
| `src/models/mcp_token.rs` | shared DTOs |

Admin UI:
| Path | Responsibility |
|---|---|
| `src/pages/admin/mcp.rs` | Token CRUD + config generator page |
| nav entry in the admin layout | link to `/admin/mcp` |

### 6.1 Tool catalogue

**Resources** (read scope, published posts only)
- `resources/list` — paginated published posts (opaque cursor; page size server-set).
- `resources/templates/list` — `post://{slug}`.
- `resources/read` — rendered Markdown of one post.

**Tools — `read`**
- `search_posts(query, limit?, cursor?)` — FTS via existing search helper; returns
  summaries + `resource_link`s to matching posts.
- `get_post(slug|id)` — published only.
- `list_tags()`.

**Tools — `write`** (also readable incl. drafts)
- `create_post`, `update_post`, `publish_post`, `trash_post`, `delete_post`.
- `list_comments`, `approve_comment`, `delete_comment`, `set_comment_status`.
- `create_tag`, `rename_tag`.
- `upload_media` (base64 → existing upload pipeline).

**Tools — `admin`**
- `get_settings`, `update_settings`.
- `run_code` (delegates to the existing code-runner execute path; admin-only).

## 7. New dependencies & env

**Cargo.toml** (added under `[features] server`) — **pinned to the 3.x beta line,
verified 2026-07-28 by a throwaway axum-0.8 probe.** The stable `rmcp 0.2.1` is far
older and LACKS `with_json_response`, `with_allowed_origins` (Origin→403),
`with_max_request_body_bytes`, protocol-version validation, and stateless-by-default —
all of which the 3.x beta ships and which this spec relies on. `transport-worker` must
be added explicitly (`LocalSessionManager` unconditionally uses `transport::worker`).
- `rmcp = { version = "=3.0.0-beta.3", optional = true, features = ["server", "macros", "transport-streamable-http-server", "transport-worker"] }`
- `aes-gcm = { version = "0.10", optional = true }` (token encryption).

**Verified integration shape** (probe at `/tmp/rmcp-probe`):
- Mount: `Router::new().nest_service("/mcp", StreamableHttpService::new(factory, LocalSessionManager::default().into(), config))`.
- Config: `StreamableHttpServerConfig::default().with_legacy_session_mode(false).with_json_response(true).with_allowed_origins([APP_BASE_URL])`.
- Auth: axum `from_fn` middleware resolves bearer → inserts `McpPrincipal{user_id,scope}` into `request.extensions_mut()`.
- Tool reads it via `Extension<http::request::Parts>` extractor → `parts.extensions.get::<McpPrincipal>()`.
- Tool result: `CallToolResult::success(vec![ContentBlock::Text(TextContent::new(s))])`.
- rmcp handles Origin→403, protocol-version 400, body-size 413 internally.

**Env vars** (document in `.env.example`):
- `MCP_TOKEN_ENC_KEY` — 32-byte AES-256 key, base64. Required when MCP is used; the
  server warns at startup if absent (tokens cannot be minted without it).

## 8. Acceptance criteria

1. `POST /mcp` with a valid `read`-scope bearer returns `tools/list` including
   `search_posts`; a `search_posts` call returns ranked published posts.
2. `resources/list` paginates published posts; `resources/read` returns one post's
   Markdown; cursor pagination round-trips.
3. A request with no/invalid/expired/revoked token is rejected (401/403).
4. A `read` token calling a `write` tool is rejected (`insufficient_scope`).
5. `write`/`admin` tokens can read drafts; `read` tokens cannot.
6. `create_post` via MCP writes a row, renders `content_html`, and invalidates the
   matching moka + SSR caches (same write-flow as the web admin).
7. Invalid `Origin` → 403; missing `MCP-Protocol-Version` → 400.
8. Admin UI lists tokens (name, scope, created, expires, last-used), reveals plaintext
   on demand, revokes, and emits copy-ready config for all 4 client formats.
9. Token DB column never stores plaintext; `token_enc` is AES-GCM ciphertext.
10. `cargo build --no-default-features --features web` compiles (WASM stubs present);
    `cargo build --no-default-features --features server` compiles;
    `cargo clippy --all-features -- -D warnings` passes.
11. `make test` (existing suite) still passes; new unit tests cover crypto, scope
    ordering, cursor pagination, and config generation (pure, DB-free).

## 9. Risks & mitigations

- **rmcp/Axum version skew** — rmcp must target axum 0.8. Verify before pinning; if
  incompatible, the fallback is a thin hand-rolled JSON-RPC-over-POST layer (larger
  effort, research says avoid). *Mitigation: spike the mount in the tracer bullet first.*
- **Indirect prompt injection** — search/snippet output reaches the model. *Mitigation:
  sanitize, separate read/write scopes, human-in-the-loop on mutating tools.*
- **Token leak = full blog takeover** — *Mitigation: encrypted-at-rest, selectable
  short lifetimes, revocation, token-keyed rate limit, Origin enforcement.*
- **WASM build breakage** — *Mitigation: every server-only symbol gets a stub; the
  tracer bullet must prove `--features web` compiles before tool flesh-out.*

## 10. Build path

Spec → tracer-bullet tickets (`/to-tickets`) → per-ticket `/implement` with cleared
context. Ticket files live under `.scratch/mcp-server/issues/`.

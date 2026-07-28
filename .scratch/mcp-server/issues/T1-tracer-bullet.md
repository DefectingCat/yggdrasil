# T1 — Tracer bullet: end-to-end MCP skeleton (read path)

> **Type:** tracer bullet. Proves the whole stack on BOTH build targets before any
> tool-surface flesh-out. Do this FIRST and get it green end-to-end.

## Blocking edges
- **Blocks:** T2 (token mgmt+UI), T3 (read tools+resources), T4 (write tools),
  T5 (admin tools), T6 (hardening).
- **Blocked by:** nothing (this is the root).

## Target files
- `Cargo.toml` — add `rmcp` + `aes-gcm` (optional, under `server` feature).
- `migrations/015_mcp_tokens.sql` — the `mcp_tokens` table (see spec §5.1).
- `src/db/migrate.rs` — register `("015", include_str!(...))` in `MIGRATIONS`.
- `src/models/mcp_token.rs` — `TokenScope` enum (Read/Write/Admin, `>=` ordering, serde),
  `McpToken`, `McpTokenSummary`, `CreateTokenResponse`.
- `src/mcp/mod.rs`, `src/mcp/crypto.rs`, `src/mcp/auth.rs`, `src/mcp/server.rs`,
  `src/mcp/router.rs` — new module (server-only impl + WASM stubs).
- `src/main.rs` — build `mcp_route` and `.merge(mcp_route)` into the app router
  (alongside existing merges at ~line 332).
- `src/lib.rs` / `src/main.rs` — `pub mod mcp;` gated by feature.
- `.env.example` — document `MCP_TOKEN_ENC_KEY`.

## Change
1. Add deps; verify `rmcp` builds against **axum 0.8** and exposes a Tower
   `StreamableHttpService` mountable via `nest_service`. If the version is incompatible,
   STOP and report — do not silently hand-roll JSON-RPC (that's a spec-level decision).
2. Migration + model (spec §5). `token_hash` = SHA-256 hex for lookup;
   `token_enc` = AES-GCM (nonce‖ct‖tag) hex.
3. `crypto.rs`: `encrypt_token(plain, &key) -> String` / `decrypt_token(enc, &key) ->
   Option<String>` using `aes-gcm`. Read key from `MCP_TOKEN_ENC_KEY` (base64 32-byte).
   `.expect()` only at LazyLock init of a parsed key is acceptable per AGENTS.md §16.
4. `auth.rs`: bearer extractor — parse `Authorization: Bearer ygg_...`, hash, DB-lookup
   the active (non-revoked, non-expired) row, return `(user_id, scope)`, bump
   `last_used_at`. Origin check → 403 (reuse CSRF trusted-origin allowlist helper).
5. `server.rs`: minimal rmcp `ServerHandler` exposing ONE tool — `search_posts(query)`
   — wired to the existing FTS search helper in `src/api/posts/search.rs`. Return
   summaries. (Full Resources/pagination is T3; here just prove dispatch works.)
6. `router.rs`: assemble `StreamableHttpService` (stateless) and return an
   `axum::Router` for `/mcp`. Merge into the app router in `main.rs`.
7. WASM stubs: every public symbol gets a `#[cfg(not(feature="server"))]` stub so
   `--features web` compiles.

## Acceptance — STATUS (2026-07-28)
- **rmcp version pinned: `=3.0.0-beta.3`** (NOT stable 0.2.1 — see spec §7). Axum-0.8
  compatibility verified by a throwaway probe: `nest_service("/mcp", StreamableHttpService)`
  mounts; auth flows via `from_fn` middleware → `request.extensions` → `Extension<Parts>`;
  real MCP `tools/list` + `tools/call` round-trip confirmed; Origin→403 confirmed.
- No-token → 401, bad Origin → 403 (rmcp built-in): **verified at probe level**.
  (Live-server token-seed smoke test deferred to T7 integration — needs a running DB.)
- `token_enc` column stores AES-GCM ciphertext (crypto.rs unit-tested: round-trip,
  tamper-fail, wrong-key-fail, nonce-distinct — 8 tests green).
- `cargo build --no-default-features --features web` ✓ (mcp module is `#[cfg(server)]`,
  so WASM build never touches rmcp/aes-gcm — no stubs needed by design).
- `cargo build --no-default-features --features server` ✓.
- `cargo clippy --all-features -- -D warnings` ✓ (added `#[allow(dead_code)]` on the
  mcp module + mcp_token model: forward-public symbols consumed by T2/T3).
- `cargo test --features server` ✓ 639 unit + 1 integration, +13 new MCP tests.
- Deferred to T7: real-client end-to-end against a live DB (T1 proved the *mechanism*
  via probe, not against the real DB-backed auth path).

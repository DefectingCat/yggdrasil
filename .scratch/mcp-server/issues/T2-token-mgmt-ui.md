# T2 — Token management server fns + admin UI

## Blocking edges
- **Blocks:** T7 (integration verification needs a usable UI).
- **Blocked by:** T1 (token model + crypto must exist).

## Target files
- `src/api/mcp_tokens.rs` — `#[server]` fns.
- `src/api/mod.rs` — `pub mod mcp_tokens;` + re-exports.
- `src/models/mcp_token.rs` — DTOs (shared with T1).
- `src/pages/admin/mcp.rs` — token management + config generator page.
- `src/router.rs` — route `/admin/mcp`.
- admin layout/nav — link to `/admin/mcp`.
- `src/mcp/config.rs` — config generation (4 formats).

## Change
1. `create_mcp_token(name, scope, lifetime)` (admin-guarded via
   `get_current_admin_user().await?`): generate opaque `ygg_...` token, AES-GCM encrypt,
   store hash + ciphertext, set `expires_at` from the preset menu
   (1d/7d/30d/90d/None). Return `CreateTokenResponse { summary, plaintext }`.
2. `list_mcp_tokens()` → `Vec<McpTokenSummary>` (no secret): name, scope, created,
   expires, last_used, revoked.
3. `reveal_mcp_token(id)` → decrypt `token_enc` → return plaintext (retrievable, per
   decision 8). Admin-guarded.
4. `revoke_mcp_token(id)` → set `revoked_at = now()`.
5. `src/mcp/config.rs`: `generate_client_configs(base_url, token)` → struct with 4
   ready-to-paste snippets: Claude Code/Cursor JSON, Cline JSON (`streamableHttp`),
   generic raw JSON, and a `claude mcp add` CLI one-liner (see research §"Client-config").
6. `/admin/mcp` page: token list table, "create token" form (name + scope dropdown +
   lifetime preset), one-time plaintext reveal + "reveal again" button, revoke button,
   and the 4 config snippets with copy buttons. Base URL from `APP_BASE_URL`.

## Acceptance
- Admin can create a token with chosen scope + lifetime; plaintext shown once and
  re-revealable; token appears in the list with correct metadata; revoke works.
- All 4 config snippets are valid JSON / shell and contain the correct bearer header.
- Non-admin calling these server fns → 401/403.
- `last_used_at` does NOT leak plaintext; plaintext never persisted unencrypted.
- Compiles on both targets; clippy clean; existing tests pass.

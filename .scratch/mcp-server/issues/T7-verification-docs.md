# T7 — Integration verification + docs

## Blocking edges
- **Blocks:** nothing (terminal ticket).
- **Blocked by:** T2 (UI), T6 (hardening).

## Target files
- `docs/CHANGELOG.md`, `.env.example`, `AGENTS.md` (architecture note), `docs/DEPLOYMENT.md`.

## Change
1. **Manual smoke test** against a running instance with a real client (Claude Code
   preferred, Cursor/Cline as available): mint a `read` token → connect → `search_posts`
   → `resources/read`; mint a `write` token → `create_post` → confirm it appears in the
   web admin with rendered `content_html`; mint an `admin` token → `run_code`. Confirm
   invalid token / bad Origin / scope-mismatch are rejected.
2. Update `docs/CHANGELOG.md` (Keep a Changelog): `feat(mcp): ...` entry.
3. `.env.example`: document `MCP_TOKEN_ENC_KEY` + MCP rate-limit vars.
4. `AGENTS.md`: add a short MCP section (transport, auth model, scope mapping, the
   `/mcp` route, feature-gating note) under the existing architecture structure.
5. `docs/DEPLOYMENT.md`: note that `/mcp` must pass through the reverse proxy and that
   `MCP_TOKEN_ENC_KEY` is required for the feature.

## Status (2026-07-28)
- Docs (CHANGELOG / .env.example / AGENTS.md / DEPLOYMENT.md): ✓ done.
- `make lint` equivalent: ✓ `cargo clippy --all-features -- -D warnings` clean.
- **Live-client smoke test DEFERRED to user**: requires a running DB instance +
  `MCP_TOKEN_ENC_KEY` env + a real MCP client (Claude Code/Cursor). The mechanism is
  proven (T1 probe verified tools/list, tools/call, Origin→403, bearer→principal
  end-to-end against rmcp directly); the remaining step is exercising it against the
  integrated app with a seeded token. Steps for the user:
  1. Set `MCP_TOKEN_ENC_KEY=$(openssl rand -hex 32)` + restart the app (runs migration 017).
  2. In `/admin/mcp`, mint a `read` token → copy the Claude Code config → `claude mcp add`.
  3. Verify `search_posts` returns published posts; mint a `write` token → `create_post`
     → confirm it appears in the web admin with rendered `content_html`.
  4. Confirm invalid token → 401, scope mismatch → invalid_request.

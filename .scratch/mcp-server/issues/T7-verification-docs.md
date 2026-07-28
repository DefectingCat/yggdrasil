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

## Acceptance
- At least one real client connects end-to-end and exercises read + write + admin paths.
- All rejection paths confirmed.
- Docs updated; no plaintext tokens committed anywhere.
- `make lint` passes.

# T6 — Hardening (rate limit, sanitization, audit, last_used_at)

## Blocking edges
- **Blocks:** T7 (verification exercises a hardened endpoint).
- **Blocked by:** T3, T4, T5 (tools must exist to harden their outputs).

## Target files
- `src/mcp/rate_limit.rs` — token-keyed governor.
- `src/mcp/auth.rs` — `last_used_at` update path, body-size cap.
- `src/mcp/server.rs` / tool outputs — snippet sanitization.
- audit log integration (tracing, or a dedicated table if the project has one — check
  existing conventions first).

## Change
1. **Token-keyed rate limit** on `/mcp` only: extract bearer → `user_id` → governor key.
   Leave the existing IP-keyed governor on the web app untouched. Add tunable
   `RATE_LIMIT_MCP_PER_SEC` / `_BURST` (match the existing env-var naming pattern).
2. **`last_used_at`** updated on each authenticated request (cheap UPDATE, or batched —
   match the session-recheck pattern).
3. **Snippet/output sanitization:** treat search + tool output as indirect-prompt-
   injection surface — strip control chars, cap length, no raw executable instructions.
   Keep read vs write tools on separate scopes (already enforced).
4. **Body-size cap** on `/mcp` POST (e.g. 1 MiB) to bound abuse.
5. **Audit logging** of mutating tool calls (token id, tool, outcome) via the project's
   existing logging convention (`tracing`).

## Acceptance
- A token exceeding its rate budget is throttled (429 / MCP rate-limit error).
- `last_used_at` advances on authenticated requests.
- Search output is sanitized (no raw control chars; length bounded).
- Oversized POST body is rejected.
- Mutating tool calls leave an audit trace.
- Both targets compile; clippy clean; existing tests pass.

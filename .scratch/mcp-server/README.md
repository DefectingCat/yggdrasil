# MCP Server — Ticket Index

Spec: `docs/mcp-spec.md` · Research: `docs/mcp-research.md` · Tickets: `issues/`

## Dependency DAG

```
T1 tracer bullet (mount+auth+crypto+1 read tool)
├─► T2 token mgmt server fns + admin UI ─┐
├─► T3 read tools + Resources ────┐       │
├─► T4 write tools ───────────────┤       │
└─► T5 admin tools ───────────────┤       │
                                  ▼       │
                          T6 hardening ◄──┤
                                  │       │
                                  ▼       ▼
                          T7 verification + docs
```

## Execution order

| # | Ticket | Depends on | Notes |
|---|--------|-----------|-------|
| T1 | Tracer bullet — end-to-end skeleton (read path) | — | **Do first.** Proves rmcp/Axum mount + both build targets. Halt if rmcp≠axum 0.8. |
| T2 | Token management server fns + admin UI | T1 | Config generation for 4 clients lives here. |
| T3 | read-scope tools + Resources (knowledge base) | T1 | Can run parallel to T4/T5. |
| T4 | write-scope tools (posts/comments/tags/media) | T1 | Reuse existing helpers; same cache invalidation. |
| T5 | admin-scope tools (settings, code runner) | T1 | runner admin-only. |
| T6 | Hardening (rate limit, sanitization, audit) | T3, T4, T5 | Token-keyed governor; injection sanitization. |
| T7 | Integration verification + docs | T2, T6 | Real-client smoke test; CHANGELOG/env/AGENTS. |

T2/T3/T4/T5 are independent after T1 — fan out. T6 waits on the tool tickets; T7 is terminal.

# T5 — admin-scope tools (settings, code runner)

## Blocking edges
- **Blocks:** T6 (hardening covers these).
- **Blocked by:** T1 (mount + auth + dispatch exist).

## Target files
- `src/mcp/tools/settings.rs`, `src/mcp/tools/runner.rs` — new.
- `src/mcp/server.rs` — register admin tools (scope-gated).

## Change
1. **Settings:** `get_settings`, `update_settings` — delegate to `src/api/settings.rs`.
   `admin` scope only. On update, invalidate settings caches + SSR.
2. **Code runner:** `run_code(language, source)` — delegate to the existing code-runner
   execute path (`src/api/code_runner/execute.rs`). `admin` scope only (decision 6).
   Return captured stdout/stderr within `CODE_RUNNER_MAX_TIMEOUT_SECS`.

## Acceptance
- `admin` token can read/update settings; change is reflected site-wide after cache
  invalidation.
- `admin` token `run_code` executes in the sandbox and returns output (skips cleanly if
  Docker daemon unavailable — match existing `require_docker()` behavior).
- `write`/`read` tokens calling these → `insufficient_scope`.
- Both targets compile; clippy clean; existing tests pass.

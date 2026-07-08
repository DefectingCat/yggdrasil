# AGENTS.md

## Workflow

- **每完成一个功能点立即提交**。Agent 自主判断提交时机——当一个逻辑完整的改动通过验证(编译通过 / 测试通过)后,无需等待用户指令,直接 `git add` + `git commit`。
- 提交粒度按"功能点"而非"文件":相关联的多文件改动合并为一个提交,不相关的改动拆成多个提交。
- 提交信息遵循现有风格:`type(scope): 简述`,正文(可选)说明动机与关键改动。常见 type:`feat` / `fix` / `docs` / `refactor` / `chore` / `perf`。
- 只在用户明确要求时才 `git push`。提交到本地即可,不主动推送。

## JavaScript 库

在 libs 目录下都是 JavaScript 库，他们的包管理器都是 pnpm。

## Development Commands

```bash
make dev           # 增量构建 4 个 libs (pnpm -r run build) + tailwindcss watch + dx serve (needs PostgreSQL, SSR_CACHE_SECS=0)
make build         # pnpm install → build-libs → highlight-css → tailwindcss → doc → dx build --release → restore-webp
make build-linux   # 客户端 + 服务端分离构建,target x86_64-unknown-linux-musl
make build-freebsd # cross-compile FreeBSD x86_64 server binary (clang + lld + sysroot, via cargo, not dx)
make freebsd-sysroot # download/extract FreeBSD base.txz → .freebsd-sysroot/ (idempotent)
make css           # one-shot Tailwind build
make css-watch     # Tailwind watch mode
make test          # cargo test + pnpm -r run test (all 4 libs: tiptap-editor / lightbox / yggdrasil-core / codemirror-editor)
make doc           # cargo doc (ayu 主题) → 拷贝到 public/doc/，随 build 发布
make doc-open      # 同 doc，生成后自动用浏览器打开（本地预览，不拷贝）
make lint          # Biome check (libs) + cargo clippy (严格模式,warning 即失败)
make fix           # biome format --write (libs) + cargo fix --allow-dirty
make clean         # cargo clean + rm public/{style.css,highlight.css,doc} + rm -rf uploads/.cache + rm -rf libs/node_modules libs/*/node_modules
```

**Build order matters**: `make build` runs `pnpm install --frozen-lockfile` (in `libs/`) → `build-libs` (`pnpm -r run build`, all 4 libs in parallel) → `highlight-css` (`cargo run --bin generate_highlight_css`) → `tailwindcss --minify` → `doc` → `dx build --release` → `restore-webp`. Do not run `dx build --release` alone.

**`restore-webp` workaround**: dx build 0.7.9 re-encodes `public/*.webp` into VP8L lossless stills (drops animation frames, 7-8× larger), contradicting the "verbatim copy" promise. `restore-webp` overwrites `.webp` in `target/dx/**/web/public/` from source `public/`. SVG/ICO are unaffected. Remove once upstream fixes it.

**JS workspace lives under `libs/`**: `libs/` is a pnpm workspace — root `libs/package.json` hoists the 4 shared devDeps (`happy-dom`/`typescript`/`vite`/`vitest`) + Biome; `libs/pnpm-workspace.yaml` lists `libs/*` packages; `libs/pnpm-lock.yaml` is the single lockfile. First-time setup: `cd libs && pnpm install`. All Makefile recipes `cd libs && pnpm ...`. Build a single lib: `make build-editor` (≈ `cd libs && pnpm --filter @yggdrasil/tiptap-editor run build`).

## Prerequisites

- Rust 1.95+ with `wasm32-unknown-unknown` target
- `dx` CLI (`cargo install dioxus-cli`)
- `tailwindcss` CLI v4 — install via `npm install -g @tailwindcss/cli` (v4 splits the CLI into its own package; the `tailwindcss` core package has no `bin`), or use the standalone binary
- `pnpm` 11+ (`libs/` is a pnpm workspace; vendored as a `devDependency` via corepack, but a global install is convenient)
- PostgreSQL running locally

**Biome** (linter + formatter for `libs/`) is a `devDependency` in `libs/package.json` — no separate install. `make lint` runs `biome check` + `cargo clippy`; `make fix` runs `biome format --write` + `cargo fix`. Config in `libs/biome.json`.

## Environment

Create `.env` (not committed):

```
DATABASE_URL=postgres://postgres:postgres@localhost:5432/yggdrasil
RUST_LOG=info
```

**Migrations run automatically at startup** — there is no `migrate.sh`. On boot `src/main.rs` calls `db::migrate::run_on_conn`, which applies `migrations/*.sql` in order (tracked in the `MIGRATIONS` array in `src/db/migrate.rs`). Before the pool is touched, `db::pool.rs::ensure_database_exists` connects to the `postgres` maintenance DB and `CREATE`s the target DB if missing (zero manual setup). The migration step is serialized across instances via an advisory lock and waits up to `MIGRATE_STARTUP_TIMEOUT_SECS` for PostgreSQL.

**Adding a migration**: create `migrations/NNN_name.sql`, then add a `(version, include_str!("./../migrations/NNN_name.sql"))` row to the `MIGRATIONS` array in `src/db/migrate.rs`. A compile-time test asserts every `.sql` file on disk has a matching row (and vice versa), so forgetting the row fails the build.

Optional tuning via env vars (all have sane defaults):

```
WEBP_QUALITY=85.0           # 0.0–100.0, clamped
WEBP_METHOD=2               # 0–6, clamped
MAX_IMAGE_DIMENSION=8192     # max single side in px, min 512, no upper limit
MAX_IMAGE_PIXELS=50000000   # max total pixels (~7k×7k), min 1M, no upper limit
RATE_LIMIT_STRICT_PER_SEC=1
RATE_LIMIT_STRICT_BURST=5
RATE_LIMIT_UPLOAD_PER_SEC=2
RATE_LIMIT_UPLOAD_BURST=15
RATE_LIMIT_IMAGE_PER_SEC=10
RATE_LIMIT_IMAGE_BURST=50
RATE_LIMIT_COMMENT_PER_SEC=1          # comment posting
RATE_LIMIT_COMMENT_BURST=5
RATE_LIMIT_UNKNOWN_PER_SEC=30         # fallback bucket when real client IP can't be determined
RATE_LIMIT_UNKNOWN_BURST=100
RATE_LIMIT_CODE_EXEC_PER_SEC=1        # code runner per-IP burst (governor: integer only, no decimals)
RATE_LIMIT_CODE_EXEC_BURST=3
RATE_LIMIT_CODE_EXEC_DAILY=50          # code runner per-IP daily cap
DB_POOL_SIZE=20             # database connection pool size
MIGRATE_STARTUP_TIMEOUT_SECS=30  # how long startup waits for PostgreSQL before giving up
STATEMENT_TIMEOUT_SECS=30   # per-query timeout; slow queries are canceled to protect the pool
SSR_CACHE_SECS=3600         # incremental SSR cache TTL (set 0 in dev)
SYSINFO_SAMPLE_SECS=0.5     # sysinfo sampling interval in seconds, supports decimals
```

Code Runner tuning (all optional, sane defaults):

```
CODE_RUNNER_ALLOW_NETWORK=false        # global network switch; AND-ed with per-language allow_network
CODE_RUNNER_MAX_CONCURRENT=4           # tokio Semaphore for in-flight containers
CODE_RUNNER_MAX_CPU_CORES=2.0          # upper clamp for cpu_cores (lower bound 0.1)
CODE_RUNNER_MAX_MEMORY_MB=1024         # upper clamp for memory_mb (lower bound 16)
CODE_RUNNER_MAX_TIMEOUT_SECS=30        # upper clamp for timeout_secs (lower bound 1)
CODE_RUNNER_MAX_OUTPUT_BYTES=1048576   # hard cap on stdout+stderr captured
CODE_RUNNER_MAX_SOURCE_BYTES=65536     # max source size accepted by StartExec
CODE_RUNNER_QUEUE_TIMEOUT_SECS=30      # how long a task waits for a container slot before failing
CODE_RUNNER_TASK_TTL_SECS=300          # DashMap task entry lifetime (gc_old_tasks)
CODE_RUNNER_LANGUAGES=python,node      # ops whitelist (must also exist in LANGUAGES registry)
DOCKER_SOCKET_PATH=/var/run/docker.sock
```

Session / security tuning:

```
COOKIE_SECURE=false         # set true/1/yes to add Secure flag to session cookie
TRUSTED_PROXY_COUNT=0       # number of reverse proxies in front of the app; used to extract real client IP from X-Forwarded-For
APP_BASE_URL=               # e.g. https://your-domain.example — trusted origin for CSRF checks on write requests; unset falls back to Host header + X-Forwarded-Proto
```

## Architecture: Conditional Compilation

Dioxus 0.7 fullstack project with **two independent gates** — the most common source of compilation errors.

| Gate                             | Applies to         | Used for                                                                                     |
| -------------------------------- | ------------------ | -------------------------------------------------------------------------------------------- |
| `#[cfg(feature = "server")]`     | Server binary only | DB, env loading, background tasks, server function bodies, highlight, WebP, caching, sysinfo |
| `#[cfg(target_arch = "wasm32")]` | WASM frontend only | localStorage, DOM APIs, web_sys calls, theme detection                                       |

**Critical**: Both default features (`web` + `server`) are enabled in `Cargo.toml`. The `dx` CLI handles feature selection during builds.

**Stub pattern**: `src/db/mod.rs` provides a `DummyPool` when `server` feature is disabled — do not remove.

**Server-only helpers**: `src/auth/password.rs`, `src/auth/session.rs`, `src/api/auth.rs`, `src/api/comments/helpers.rs`, and several model helper methods are gated with `#[cfg(feature = "server")]` because they are only called from server function bodies, which are stripped in WASM builds.

**Server-only dependencies**: Crates that are only used behind `#[cfg(feature = "server")]` (e.g., `argon2`, `uuid`, `regex`, `pulldown-cmark`, `rand`, `http`, `sha2`, `hex`, `sysinfo`, `sqlparser`, `dashmap`, plus the rest of the server stack) are declared as `optional = true` in `Cargo.toml` and enabled only through the `server` feature. They are not compiled into the WASM frontend. The `generate_highlight_css` binary is `required-features = ["server"]` (uses `syntect`).

**Shared types that compile on both targets**: bridges like `src/tiptap_bridge.rs` and `src/codemirror_bridge.rs` keep shared structs (e.g. `UploadsInFlight`, `SqlSchema`/`SqlTable`) outside cfg gates, while wasm-bindgen externs + `EditorHandle` live in inner `#[cfg(target_arch = "wasm32")] mod wasm`. Similarly `src/sysinfo_sampler.rs` exposes `SystemSnapshot` on both targets but gates the actual sampler + `RwLock` behind `server`.

## Dual API Architecture

The server exposes two distinct API patterns:

1. **Dioxus server functions** (`#[server(Name, "/api")]` in `src/api/`) — auto-routed, callable from both client and server Rust. Spread across `src/api/auth.rs`, `src/api/posts/`, `src/api/comments/`, `src/api/settings.rs`, and `src/api/database/`.

2. **Axum routes** (registered in `src/main.rs`) — manual `axum::Router` for endpoints that don't fit the server-function model:
   - `POST /api/upload` — image upload (multipart, auth-required, rate-limited)
   - `GET /uploads/{*path}` — image serving with on-the-fly resize/rotate/convert (query params: `w`, `h`, `thumb`, `rotate`, `format`, `quality`)

## Server Module Structure

```
src/api/          — server functions + Axum handlers
  auth.rs         — login, register, session validation
  comments/       — comment CRUD + approval/spam/trash server functions
  database/       — /admin/system backend (status/system_status/sql_console/schema/export/backup/tasks)
  markdown.rs     — Markdown→HTML rendering (pulldown-cmark + ammonia sanitization)
  image.rs        — image serving with processing pipeline + disk+memory cache
  upload.rs       — image upload, auto-converts to WebP
  rate_limit.rs   — governor-based rate limiting (6 tiers: strict/upload/image/comment/unknown/code_exec)
  settings.rs     — site settings server functions (trash retention, etc.)
  slug.rs         — URL slug generation
  posts/          — CRUD server functions for blog posts
  code_runner/    — runnable code-block server functions + data structures (see Code Runner section)
src/auth/         — password hashing (Argon2) + session token management
src/bin/          — generate_highlight_css (build-time CSS generation)
src/cache.rs      — moka future-based caches for posts, tags, stats + cache_stats()
src/components/   — Dioxus UI components
src/context.rs    — shared Dioxus context/state
src/db/           — PostgreSQL pool (deadpool-postgres, LazyLock global) + migrate.rs + pool.rs (ensure_database_exists)
src/hooks/        — shared Dioxus hooks
src/models/       — Post, User, Tag data models
src/pages/        — route page components (frontend + admin)
src/router.rs     — Dioxus Router route definitions
src/ssr_cache.rs  — SSR generation invalidation state (server feature only)
src/sysinfo_sampler.rs — host metrics snapshot; SystemSnapshot on both targets, sampler+RwLock server-only
src/tasks/        — background tokio tasks (session cleanup)
src/theme.rs      — light/dark theme with SSR cookie + WASM localStorage
src/webp.rs       — zenwebp encode/decode (image crate has no WebP)
src/tiptap_bridge.rs    — wasm-bindgen bindings for Tiptap editor
src/codemirror_bridge.rs — wasm-bindgen bindings for CodeMirror editor (mirrors tiptap_bridge)
src/infra/        — Docker execution layer + runner config (server-only); see Code Runner section
```

## Code Runner (` ```lang runnable ` code blocks)

Readers can execute fenced code blocks in isolated Docker containers; authors get a `/admin/runner` trial sandbox. Three-layer architecture, all Docker interaction gated behind `#[cfg(feature = "server")]`:

- **Execution layer** (`src/infra/docker.rs`, server-only): `bollard` client over Unix socket (`DOCKER_CLIENT` LazyLock). `run_in_container` creates a read-only-rootfs container (tmpfs `/code`,`/tmp`,`/run`; cpu/memory/pids/ulimits cap; `cap_drop=ALL`; `no-new-privileges`; non-root `1000:1000`; `network_mode` none/bridge), injects source via stdin, waits with timeout, captures stdout/stderr (truncated to `output_bytes`), inspects OOM, force-removes via `ContainerGuard` Drop. `src/infra/runner_config.rs` (`RUNNER_CONFIG`) reads all `CODE_RUNNER_*` env vars; `clamp_limits` AND-merges request overrides × language `allow_network` × global switch.
- **API layer** (`src/api/code_runner/`): shared `ExecRequest`/`ExecResult`/`ExecStatus`/`ExecTask` structs (compile on both targets); `progress.rs` = DashMap task registry + `gc_old_tasks`; `languages.rs` = `LANGUAGES` registry + `parse_fence_info`; `execute.rs` = `StartExec`/`GetExecResult` server functions (double rate-limit → whitelist → size check → enqueue → `tokio::spawn` + `RUNNER_SEMAPHORE` concurrency cap → clamp → run_in_container). System errors are sanitized to 「系统暂时不可用」 for anonymous callers; full errors in server logs only.
- **Markdown/render layer**: a fenced block ` ```python runnable {...overrides} ` renders to `<pre data-runnable="true" data-lang="python" data-overrides="..." data-source="...">` (sanitizer whitelists these 4 attrs on `<pre>`). `PostContent` (`src/components/post/post_content.rs`) splits `content_html` into `Html`/`Runnable` fragments so each runnable block renders as a real `<CodeRunner>` vdom element (no manual DOM mutation → no hydration conflict). `CodeRunner` component polls `GetExecResult` via WASM-friendly `sleep_ms`.

**Critical WASM-visibility rule**: `code_runner/execute.rs` is **not** cfg-gated (server functions must be visible to the client), but every server-only `use`/static inside it is individually `#[cfg(feature = "server")]`. The shared `use` of `ExecRequest`/`ExecTask` (used in signatures) stays ungated. `code_runner/languages.rs` and `code_runner/progress.rs` (pure server helpers) are wholly gated. Mirrors the `posts/` module convention.

**Governor 0.8 caveat**: `Quota::per_day` does not exist; `CODE_EXEC_DAILY_LIMITER` uses `Quota::with_period(24h).allow_burst(daily)`. `RATE_LIMIT_CODE_EXEC_PER_SEC` must be an integer (governor's `per_second` takes `NonZeroU32`; decimals in `.env` fall back to the default 1).

**Runner images** (`docker/`): `build-runners.sh` builds `yggdrasil-runner-base` → `yggdrasil-runner-python` → `yggdrasil-runner-node`; tags must match `LANGUAGES` image fields. Python image symlinks `python`→`python3` to match `run_cmd`. `runner.toml` files are image self-descriptions only — runtime config is the Rust `LANGUAGES` registry + `CODE_RUNNER_*` env, not parsed from toml.

## Frontend Lib Subprojects

Four Vite-built IIFE libraries under `libs/`, managed as a **pnpm workspace** (single `libs/pnpm-lock.yaml`, shared `libs/tsconfig.base.json` + `libs/biome.json`, hoisted devDeps). Built artifacts go to `public/<name>/` — **do not edit `public/<name>/` files; they are build artifacts**. Each `build` script is `tsc --noEmit && vite build` (type-check before bundle). Output is IIFE because Dioxus `[web.resource] script` injects bare `<script src>` without `type="module"` support. Registered globally in `Dioxus.toml` `script`/`style` arrays.

| Lib                       | Output dir                                                   | Exposes                                                                                                       | Wiring                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `libs/tiptap-editor/`     | `public/tiptap/` (`editor.js`/`.css`/`.map`)                 | `window.TiptapEditor`                                                                                         | wasm-bindgen via `src/tiptap_bridge.rs` — injects `Closure` callbacks (`onUpdate`/`onReady`/`onUploadEvent`/`onImageUpload`) into `TiptapEditor.create`, holds instance + closures in `EditorHandle` (Drop → `destroy()`). No `js_sys::eval`, no `window` globals, no polling.                                                                                                |
| `libs/codemirror-editor/` | `public/codemirror/` (`editor.js`/`.map`, **no CSS**)        | `window.CodeMirrorEditor` (object literal `{ create }`) + `window.EditorOptions` (class, survives TS erasure) | `src/codemirror_bridge.rs` mirrors tiptap — `get_module()` uses `Reflect::get` + `unchecked_into` (object literal, NOT a constructor extern). Themes are JS `Extension`s from `@catppuccin/codemirror` (Latte/Mocha), hot-swapped via `Compartment.reconfigure`.                                                                                                              |
| `libs/lightbox/`          | `public/lightbox/` (`lightbox.js`/`.css`/`.map`)             | self-initializing IIFE                                                                                        | **Not** wasm-bindgen. `src/components/post/post_content.rs` sets `window.__lightboxSelectors` before load; IIFE tail reads it and self-initializes. Direct fallback call if already loaded.                                                                                                                                                                                   |
| `libs/yggdrasil-core/`    | `public/yggdrasil-core/` (`yggdrasil-core.js`/`.css`/`.map`) | `window.__initPostContent`, `window.__startThemeTransition`                                                   | Designated home for all new core JS — add here, not to `public/js/`. Rust calls entry points via `js_sys::Reflect::get` + `Function::apply` (no `js_sys::eval`), silently no-oping if undefined. Theme reveal uses View Transitions API (`startViewTransition` + `@keyframes tt-expand` `clip-path` expand); falls back to instant switch when VT / `prefers-reduced-motion`. |

Run a single lib's tests: `cd libs && pnpm --filter @yggdrasil/<name> test` (Vitest + happy-dom). Watch mode: append `-- test:watch`.

## Database Management (`/admin/system`)

Admin area at `/admin/system` (menu "系统") with 5 tabs: 数据库状态 / 服务器状态 / SQL 控制台 / 数据导出 / 备份恢复. All gated by `get_current_admin_user` (admin-only). Backend in `src/api/database/` (status/system_status/sql_console/schema/export/backup/tasks), page in `src/pages/admin/system.rs`.

- **SQL 控制台** is full read-write with 4 guards: (1) `sqlparser` AST gates — `DROP DATABASE`/`DROP SCHEMA`/`CREATE DATABASE` absolutely forbidden (string pre-check); `DROP`/`TRUNCATE`/`ALTER` require a `confirm_dangerous` checkbox; (2) `UPDATE`/`DELETE` without `WHERE` rejected; (3) `STATEMENT_TIMEOUT_SECS` query timeout (pool-level GUC); (4) frontend write-confirm dialog. Multi-statement disabled by default. Results capped at 500 rows.
- **备份恢复** uses `dashmap` task-progress table; `create_backup`/`restore_backup` return a task_id immediately and poll `get_task_progress`. Backup prefers `pg_dump` (full, incl. schema), falls back to per-table `COPY TO STDOUT` (data only) when `pg_dump` is unavailable. Backup files carry a `-- YGGDRASIL BACKUP v1` signature header; restore rejects non-system files. `backups/` is gitignored and served only via `GET /api/database/backups/{filename}` (admin-gated, path-allowlist).
- **服务器状态** uses `sysinfo` (optional, server feature) with a background sampler (`SYSINFO_SAMPLE_SECS`, default 0.5s) writing to a `RwLock<SystemSnapshot>`; server functions read the snapshot (zero sampling cost), so frontend can poll high-frequency. `src/cache.rs` exposes moka hit-rate via `AtomicU64` hit/miss counters per cache + `cache_stats()`.

## Syntax Highlighting Pipeline

- `themes/` contains Catppuccin Latte (light) and Mocha (dark) `.tmTheme` files
- `syntaxes/` has custom Sublime syntax definitions (Kotlin, Swift)
- `src/bin/generate_highlight_css.rs` generates `public/highlight.css` with class-based rules scoped under `.md-content pre code`, with `.dark` prefix for dark mode
- `src/highlight.rs` uses syntect at runtime for code block highlighting
- All gated behind `#[cfg(feature = "server")]`

## Auth & Session

- **Registration**: first user becomes `admin`; subsequent registrations rejected with `"Registration is closed"`
- **Login**: sets an HttpOnly cookie via `FullstackContext::add_response_header`
- **Session validation**: `get_current_user` reads `session` cookie, queries `sessions` + `users` tables
- **Background cleanup**: `tasks::session_cleanup::run_cleanup()` deletes expired sessions every hour

## Caching

- **Post/tag caches** (`src/cache.rs`): moka future-based, TTL varies by data type (60s–600s). Invalidated on writes.
- **Image processing cache** (`src/api/image.rs`): two-tier — in-memory moka cache + disk cache in `uploads/.cache/`. Keyed by path + query params.
- **SSR cache** (`IncrementalRendererConfig` in `src/main.rs`): default TTL `SSR_CACHE_SECS` (3600s prod, 0 in `make dev`); invalidation generation tracked in `src/ssr_cache.rs`.

## Testing

```bash
make test # cargo test (Rust) + pnpm -r run test in all 4 libs
make lint # Biome check (libs) + cargo clippy --all-targets --all-features -- -D warnings
dx check  # Dioxus type-check (catches component/Router issues)
```

**Must verify both targets**: This is a fullstack project — the server binary and the WASM frontend are two separate compilation targets with different features (`--all-features` enables `web`+`server`; the WASM bundle uses `--no-default-features --features web`). A change that compiles on one target can fail on the other. **`cargo build --all-features` alone is NOT sufficient** — it only builds the server binary. Before considering work done, compile the WASM target too:

```bash
cargo build --all-features                                        # server target (native)
cargo build --target wasm32-unknown-unknown --no-default-features --features web  # WASM frontend
```

`dx check` does Dioxus-level type-checking but does NOT run a full `cargo build` for the WASM target, so it can miss borrow-checker / move / lifetime errors that only surface in the frontend bundle. `dx serve` / `make dev` do the real WASM compile — run them (or the explicit `cargo build --target wasm32-...` above) before declaring a task complete.

Common target-mismatch traps:

- `#[cfg(target_arch = "wasm32")]` code invisible to server build → `web_sys` / `js_sys` references only resolve on WASM.
- `let mut x = use_signal(...)` flagged as `unused_mut` on server (where the `.set()` calls live inside stripped wasm blocks) but **required** on WASM. Use `#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]` on the function/item rather than deleting `mut`.
- `use` imports placed inside a `#[cfg(target_arch = "wasm32")] {}` block are invisible to the server build; imports at module top level are visible to both. When an import is only needed on one target, gate the `use` line itself.

Most Rust tests use `#[cfg(all(test, feature = "server"))]` — they only run when the server feature is active (which is the default). No integration tests requiring a database connection; the migration `.sql`↔`MIGRATIONS`-array parity is enforced by a compile-time test. The 4 `libs/` subprojects run their own vitest suites (Vitest + happy-dom).

## Image Processing Constraints

- The `image` crate is configured **without** WebP support (`default-features = false, features = ["jpeg", "png", "gif"]`). Do not add WebP to the image crate features.
- All WebP encode/decode goes through `zenwebp` via `src/webp.rs`.
- Upload pipeline auto-converts non-GIF/non-WebP images to WebP, keeping original format if WebP is larger.
- Image serving supports on-the-fly resize (`w`, `h`), thumbnail (`thumb=WxH`), rotation (90/180/270), and format conversion.

## Build Artifacts (gitignored)

- `public/style.css` — Tailwind output
- `public/highlight.css` — generated by `generate_highlight_css` binary
- `public/{tiptap,codemirror,lightbox,yggdrasil-core}/` — Vite build outputs
- `public/doc/` — cargo doc output (copied by `make doc`)
- `/dist`, `/.dioxus`, `/target`, `/static`
- `libs/node_modules/` + `libs/*/node_modules/` — pnpm workspace store + symlinks
- `uploads/.cache/` — image processing disk cache
- `backups/` — admin DB backup files
- `.freebsd-sysroot/` — FreeBSD cross-compile sysroot (machine-local)

## Notes

- `rand` is optional and only enabled by the `server` feature; it is not compiled into the WASM frontend.
- `#[allow(unused_mut, unused_variables)]` on `Write` component is intentional — `mut` signals are used in `#[cfg(target_arch = "wasm32")]` blocks stripped in server builds.
- Release profile: `panic = "abort"` (drops WASM unwind metadata; server errors go through `Result` + `?`, process crashes restarted by systemd/k8s).

## Pitfall Log (踩坑记录)

Recurring traps that have cost real debugging time. Read before touching the relevant area; add new entries when you hit a non-obvious failure.

### Custom hooks that own resources (use_hook + use_effect + use_drop)

When writing a hook that registers a side effect (e.g. `src/hooks/event_listener.rs`), the `use_effect` callback is typed `FnMut` and may run more than once. But the things you move into it are often `FnOnce` (an init/acquire closure) or need to be moved onward into a `Closure::wrap` event handler. Naively `move ||`-ing them in triggers **E0507 cannot move out of captured variable in an FnMut closure** and **E0310 the parameter type may not live long enough**.

Fix pattern (used in `event_listener.rs`): wrap the `FnOnce`/`FnMut` args in `Option`, `take()` them on the first effect run, and add `'static` bounds to the relevant generic params (`A: FnOnce() -> Option<T> + 'static`). This consumes each captured value exactly once without violating `FnMut`.

Also: `use_hook` / `use_effect` / `use_drop` come from `dioxus::prelude`. If you scope an import to a `#[cfg(target_arch = "wasm32")]` block, the `use dioxus::prelude::*;` must live **inside that block** too — the server (no-wasm) variant of the hook is a no-op and must not reference them.

### Single-target verification is a lie

The single most repeated mistake: running only `cargo build --all-features` (server) and shipping, then `dx serve` blowing up on the WASM bundle. The WASM target compiles with different features and surfaces different borrow/move errors. **Always compile both targets** (see Testing section) before marking work done. `dx check` is a fast Dioxus type-check, not a substitute for `cargo build --target wasm32-unknown-unknown`.

### `mut` bindings needed only on WASM

`let mut x = use_signal(...)` produces `unused_mut` on the server build when every `.set()` lives inside a `#[cfg(target_arch = "wasm32")]` block. Don't delete `mut` — it's required on the WASM build. Use `#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]` on the enclosing fn/struct. This is the project's established convention (see `Write` component, `use_paginated`, etc.).

### Placing util functions: check the feature gate of the target module

`src/utils/text.rs` is `#[cfg(feature = "server")]`-gated and pulls in `regex` (an optional, server-only dep). A frontend-reachable function (e.g. `escape_html`, called from a `#[component]`) **cannot** move there — the WASM build would fail on the missing `regex` crate. Use an ungated module (`src/utils/html.rs`, `src/utils/time.rs`) for code that must compile on both targets. Verify the destination module's cfg before moving anything into it.

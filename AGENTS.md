# Repository Guidelines

Yggdrasil is a fullstack blog/CMS built with **Dioxus 0.7**. A single Rust crate (`yggdrasil`, Cargo edition 2021) compiles to **two targets from one codebase**: a WASM frontend (feature `web`) and a native Axum server (feature `server`). Stack: PostgreSQL via tokio-postgres + deadpool, Tailwind CSS v4, Argon2 passwords, moka cache, syntect code highlighting, katex-rs math, mimalloc allocator. The blog also runs user-submitted code in isolated Docker containers.

## Architecture & Data Flow

**The `server` feature gate is the central organizing principle.** Nearly every module gates real DB/IO/Axum logic under `#[cfg(feature = "server")]` and provides a compiling stub under `#[cfg(not(feature = "server"))]` for the WASM build. `default = ["web", "server"]` (fullstack); override to build one target only (see Development Commands).

- **Two endpoint kinds**: (1) Dioxus server functions `#[server(Name, "/api")]` (auth, posts, comments, settings, database, code_runner); (2) manual Axum routers merged into the app router in `src/main.rs` (upload, image serving, health, SSE stream).
- **Boot** (`src/main.rs`, server branch): dotenvy → tracing → build_info → hard-check `DATABASE_URL` → `validate_database_url()` → CSRF warn → a *throwaway* multi-thread tokio runtime runs `ensure_database()` + migrations + port pre-probe, then is dropped → `dioxus::server::serve` returns the Axum router with background tasks (session cleanup, post purge, image-cache cleanup, IP purge, sysinfo sampler). mimalloc is the global allocator under `cfg(all(feature="server", not(wasm32)))`.
- **Read flow** (e.g. `GET /post/:slug`): middleware stack `[ssr_generation → add_cache_control → csrf → optional compression → 30s Timeout → admin_guard]` → Dioxus IncrementalRenderer checks persisted `static/<route>/index/<hash>.html` (TTL `SSR_CACHE_SECS`, default 3600s). HIT → serve cached HTML. MISS → SSR renders `PostDetail` → `use_server_future(get_post_by_slug(slug))` → Dioxus deserializes to a server fn call → `cache::get_post_by_slug` (moka, 600s TTL) → miss → `get_conn()` from deadpool pool → query → cache set → return.
- **Write flow** (e.g. create post): admin client → POST `/api/CreatePost` → CSRF validates Origin → `get_current_admin_user()` → validate → `spawn_blocking(render_markdown_enhanced)` → BEGIN TXN → INSERT post + `sync_tags` → COMMIT → invalidate matching moka caches **and** `ssr_cache::invalidate_ssr_*` (physical dir deletion) → return.
- **Auth**: cookie-session (HttpOnly, SameSite=Lax, optional Secure), Argon2 hashing in `spawn_blocking`, moka-cached sessions re-checked against `users.session_generation` on every hit. First registered user becomes admin (atomic `INSERT ... ON CONFLICT`).
- **Code execution**: ` ```lang runnable ``` ` blocks and `/admin/runner` run code in Docker containers (bollard, `src/infra/docker.rs`) — read-only rootfs + tmpfs `/code`, UID 1000, resource/cap-limited, `ContainerGuard` cleanup. SSE streams output at `GET /api/exec/stream`.

## Key Directories

- `src/` — Rust source (single crate). `main.rs` (entry), `router.rs` (Dioxus routes), `middleware.rs` (axum layers: compression, cache-control, admin_guard), `cache.rs` (moka, many domain caches with distinct TTLs), `ssr_cache.rs` (physical SSR cache invalidation), `theme.rs` (light/dark/system), `highlight.rs` (syntect), `context.rs` (`UserContext` global login state).
  - `src/api/` — endpoints: `auth.rs`, `posts/` (create/update/delete/trash/list/read/search/stats/tags/rebuild/helpers/types), `comments/`, `settings/`, `database/` (admin: console/export/backup), `code_runner/`, `upload.rs`, `image.rs`, `health.rs`, `sse`. Cross-cutting: `error.rs` (`AppError`), `csrf.rs`, `rate_limit.rs`, `sanitizer.rs`, `slug.rs`, `markdown.rs` (`render_markdown_enhanced`), `katex.rs`.
  - `src/db/` — `pool.rs` (`DB_POOL: LazyLock<deadpool>`, `get_conn()` runtime fast-fail, `get_conn_for_startup()` retry), `migrate.rs` (`MIGRATIONS` array + runner), `retry.rs`, `mod.rs` (`format_with_sources`, `DummyPool` stub).
  - `src/models/` — `post.rs`, `user.rs`, `comment.rs`, `settings.rs` (serde DTOs shared across SSR/cache/API).
  - `src/auth/` — `password.rs` (Argon2), `session.rs` (UUID token, SHA-256 `hash_token`, cookie build/parse).
  - `src/components/` — Dioxus components (layouts, header/nav/footer, post/comments/code_runner/skeletons/forms/ui atoms).
  - `src/pages/` — route components. **`post_detail.rs` header docs are the canonical guide for `use_server_future` + route-subscription gotchas — read before editing pages.**
  - `src/tasks/` — server-only background loops spawned in `serve()`.
  - `src/infra/` — `docker.rs` (bollard), `runner_config.rs`.
  - `src/hooks/`, `src/utils/` — query/event hooks; text/time/html helpers.
  - `src/bin/generate_highlight_css.rs` — build-tool binary (regenerates `public/highlight.css` from syntect themes; `required-features = ["server"]`).
  - `src/*_bridge.rs` — wasm-bindgen bridges for JS editors/terminal (`tiptap_bridge`, `codemirror_bridge`, `xterm_bridge`).
- `libs/` — pnpm JS workspace, packages named `@yggdrasil/*`. Each builds to a self-contained IIFE bundle written **directly into `public/<dir>/`** and consumed by the Rust side via window globals (`js_sys::Reflect::get` on object-literal modules, or global `__init*` functions via a typed `invoke_optional_global` helper).
  - `tiptap-editor` → `public/tiptap/` (rich-text Markdown editor), `codemirror-editor` → `public/codemirror/` (code-runner source editor), `lightbox` → `public/lightbox/`, `xterm-terminal` → `public/xterm/`, `yggdrasil-core` → `public/yggdrasil-core/`, `mermaid-renderer` (dynamically script-injected by yggdrasil-core on viewport visibility), `shared` (cross-lib constants: `ThemeName`, `THEME_CHANGE_EVENT` — inlined into each IIFE, not bundled).
- `migrations/` — 14 numbered SQL files (`NNN_desc.sql`); each must also be registered in the `MIGRATIONS` array in `src/db/migrate.rs` (enforced by a compile-test).
- `syntaxes/` — `.sublime-syntax` definitions (JSX/Kotlin/Swift/TSX/TypeScript/Vue/Zig); embedded via `include_str!` at compile time.
- `themes/` — Catppuccin Latte (light) / Mocha (dark) `.tmTheme` for syntect.
- `docker/` — `Dockerfile` (app), `build-runners.sh` + `runner-base/` + `runner-{python,node,go,rust,bun}/` (sandbox images).
- `docs/` — `DEPLOYMENT.md`, `test-markdown.md` (rendering test fixture). `DEVELOPMENT.md` (perf benchmarking + highlighting guide). `CHANGELOG.md` (Keep a Changelog v1.1.0, SemVer; current `0.5.0`).
- `scripts/` — `migrate.sh` (manual DB migration runner; companion to the built-in startup migrator), `xun.fish` (full-deploy pipeline to the `xun` server — build all images, scp, rolling-restart `app` only, verify).
- `static/` — Dioxus IncrementalRenderer persists SSR HTML here at runtime (gitignored output, not source).

## Development Commands

Prerequisites: Rust 1.95+, `wasm32-unknown-unknown` target, `dx` CLI (v0.7.9), `tailwindcss` CLI v4, PostgreSQL, Node 20+ / pnpm.

```bash
# Dev server (builds libs + highlight.css + katex.css first, assumes node_modules present)
make dev

# Full release build (client WASM + native server)
make build

# Linux cross-build (musl static binary)
make build-linux

# CSS
make css           # input.css -> public/style.css (one-shot)
make css-watch     # with --watch

# Lint (JS Biome + Rust clippy, no writes)
make lint

# Auto-fix (Biome -> cargo fix -> cargo fmt -> dx fmt)
make fix

# Tests
make test          # == cargo test --features server
cargo test --features server highlight_code_swift -- --nocapture   # single highlight test w/ output

# Docs (rustdoc, --no-deps --document-private-items, ayu theme) -> public/doc/
make doc
make doc-open

# Build JS libs
make build-libs
make build-editor      # pnpm --filter @yggdrasil/tiptap-editor run build (also codemirror/lightbox/core/xterm/mermaid)

# Regenerate highlight.css (only when adding new syntect scope types)
cargo run --features server --bin generate_highlight_css

# WASM-only build (e.g. to check the web target)
cargo build --no-default-features --features web
# Server-only build (as the Dockerfile does)
cargo build --no-default-features --features server

# Database
DATABASE_URL=postgres://postgres:postgres@localhost:5432/yggdrasil ./scripts/migrate.sh   # manual; also auto-runs on server startup

# Docker
make docker              # native arch, load into local daemon
make docker-amd64        # x86_64 via QEMU (Apple Silicon)
make docker-apple        # x86_64 via Apple Container CLI (macOS 26+)
make docker-multiarch IMAGE=ghcr.io/owner/yggdrasil:latest   # amd64+arm64, push to registry
```

## Important Configuration

**Feature model** (`Cargo.toml`): `default = ["web", "server"]`. `web` = `dioxus/web` (WASM); `server` = all native deps (tokio, axum, tokio-postgres, deadpool, argon2, moka, syntect, katex-rs, mimalloc, governor, bollard, …). Most deps are `optional = true` and gated behind the `server` feature list. Release profile: `opt-level=3`, `lto="thin"`, `codegen-units=1`, `strip=symbols`, **`panic="abort"`** (shed WASM unwind metadata; server relies on systemd/k8s restart — design around `Result + ?`, never panic-driven control flow).

**`.cargo/config.toml`**: sets `--cfg getrandom_backend="wasm_js"` for `wasm32-unknown-unknown` only (workaround for a Dioxus 0.7.9 cfg leak into the server build). musl/freebsd linker blocks are commented templates.

**`build.rs`**: injects `YGG_BUILD_GIT_DESCRIBE/HASH/COMMIT_DATE` + rustc version + build time via `cargo:rustc-env` (read by `src/build_info.rs` through `env!`). 3-tier fallback: env var → local `git` → `"unknown"`. `rerun-if-changed=.git/HEAD` + `.git/index`. std-only (no build-deps).

**Self-contained binary**: migrations (`src/db/migrate.rs` `include_str!`), custom syntaxes (`src/highlight.rs` `include_str!`), and `public/highlight.css` (pre-generated at build time) are embedded — the runtime `scratch` image needs only the binary + `public/` + `uploads/`.

**Key env vars** (see `.env.example` for the full ~45-var reference; no mailer, no `LISTEN_ADDR` — uses `IP`/`PORT`, no `UPLOAD` dir env, no session-lifetime/ADMIN env):

| Category | Var | Purpose (defaults) |
|---|---|---|
| Database | `DATABASE_URL` | PostgreSQL connection string (required) |
| Database | `DB_POOL_SIZE` | deadpool pool size (20) |
| Database | `STATEMENT_TIMEOUT_SECS` | per-query SQL timeout (30) |
| Database | `MIGRATE_STARTUP_TIMEOUT_SECS` | startup DB-connect retry window (30) |
| Server | `RUST_LOG` | tracing filter (`info`) |
| Server | `IP` / `PORT` | bind address (set in Dockerfile `0.0.0.0:3000`) |
| Server | `DIOXUS_PUBLIC_PATH` | public assets path (Dockerfile `/app/public`) |
| Perf | `SSR_CACHE_SECS` | SSR page cache TTL (3600) |
| Perf | `COMPRESSION_ALGORITHMS` | response compression — gzip/brotli/deflate/zstd/`all`/`off` (**off**) |
| Perf | `TOKIO_WORKER_THREADS` | tokio workers (read by runtime, not app code) |
| Perf | `SYSINFO_SAMPLE_SECS` | `/admin/system` metric interval (0.5) |
| Security | `APP_BASE_URL` | CSRF trusted origin (prod strongly recommended; else Host-header fallback) |
| Security | `COOKIE_SECURE` | add `Secure` to session cookie (false) |
| Security | `TRUSTED_PROXY_COUNT` | reverse-proxy hop count for real-IP from XFF (0) |
| Security | `EXPOSE_VERSION_HEADERS` | attach Server/X-Yggdrasil-Version/Git headers (true) |
| Security | `MAX_SESSIONS_PER_USER` | concurrent-session cap w/ LRU evict (5) |
| Images | `WEBP_QUALITY` / `WEBP_METHOD` | WebP encode quality (85) / method (2) |
| Images | `MAX_IMAGE_DIMENSION` / `MAX_IMAGE_PIXELS` | max edge px (8192) / total pixels (50M) |
| Images | `IMAGE_DISK_CACHE_MAX_MB` / `_MAX_AGE_HOURS` | `uploads/.cache` cap (1024) / retention (168 = 7d) |
| Rate limit | `RATE_LIMIT_{STRICT,UPLOAD,IMAGE,COMMENT,CODE_EXEC,UNKNOWN}_PER_SEC/_BURST` | governor buckets keyed by client IP |
| Runners | `CODE_RUNNER_ALLOW_NETWORK` / `_MAX_CONCURRENT` / `_MAX_CPU_CORES` / `_MAX_MEMORY_MB` / `_MAX_TIMEOUT_SECS` / `_MAX_OUTPUT_BYTES` / `_MAX_SOURCE_BYTES` | sandbox limits |
| Runners | `CODE_RUNNER_LANGUAGES` | optional allow-list (default: all registered) |
| Runners | `DOCKER_SOCKET_PATH` | docker.sock for bollard (`/var/run/docker.sock`) |

**Production deployment** (see `docs/DEPLOYMENT.md`): the app does NOT do TLS — a reverse proxy (nginx/Caddy) is mandatory. **MUST set** `APP_BASE_URL`, `COOKIE_SECURE=true`, `TRUSTED_PROXY_COUNT` (exact proxy hop count — a wrong value lets attackers spoof XFF to bypass rate limits or makes all users share one proxy-IP bucket). nginx: `client_max_body_size 12m` (app hard-limits 10 MiB), `proxy_read/send_timeout 360s` (image transcoding up to 300s). Bind `127.0.0.1:3000:3000`, not `0.0.0.0`. Health: `/healthz` (liveness), `/readyz` (readiness, `SELECT 1`).

## Code Conventions & Common Patterns

1. **Dual-target gating.** Any code touching DB/IO/Axum must gate impl under `#[cfg(feature = "server")]` and provide a compiling `#[cfg(not(feature = "server"))]` stub. Never put server-only deps in code reachable by the web build. The `DummyPool` stub in `src/db/mod.rs` exists for this — do not delete it.
2. **Server functions.** `#[server(FnName, "/api")] pub async fn name(args...) -> Result<T, ServerFnError>`. Args/return serde-serializable. Re-export from the module's `mod.rs` (`pub use create::create_post;`).
3. **Error handling.** Never `?` a raw DB error into `ServerFnError`. Use `AppError` constructors (`db_conn`/`query`/`tx`) which log the full chain via `db::format_with_sources` but expose a generic message — they never leak SQL. Map domain failures via `AppError::Unauthorized/Forbidden/NotFound/BadRequest/Internal` then `.into()`. Validation/business rejections return `Ok(Response{success:false,...})`, **not** `Err`.
4. **Component purity (Dioxus 0.7).** `#[component]` bodies and `rsx!` must be **pure** — no `signal.set`, `spawn`, DOM calls, or side effects in the render body. Derive data inline or via signals; do effects in `use_effect`. Don't store derivable data in `use_signal`. (See the `dioxus-render-purity` skill.)
5. **Async data in pages.** Use `use_server_future(move || { ... })?`. To re-run on route-param change you MUST read the router state **inside the closure** via `router().current::<Route>()` (it subscribes via `ReactiveContext`) — a moved `String` prop is a frozen snapshot that won't re-trigger. To force a child remount on identity change (e.g. slug), wrap it in a single-element `for x in std::iter::once(...) { Comp { key: "{x}" } }` — a bare `key` on a non-list element is ignored by Dioxus's diff. See `src/pages/post_detail.rs` header docs.
6. **Auth guard.** Every admin server fn starts with `let user = get_current_admin_user().await?;`. The SSR `admin_guard` middleware is a fast-path 302 (fail-OPEN on DB error); the client `AdminLayout` is the backstop. Don't rely solely on the middleware for security decisions in server fns.
7. **CPU-bound work** (Argon2, syntect/markdown render) MUST go in `tokio::task::spawn_blocking` — never on the async worker.
8. **Caching.** Read-through on reads (`cache::get` → miss → db → `cache::set`); on writes call the matching `cache::invalidate_*` **and** `ssr_cache::invalidate_ssr_*` before returning. Use the `CacheKey` enum; don't hand-roll keys. Note: `ssr_cache::GLOBAL_GENERATION` / `X-SSR-Generation` is **observability only** — real SSR freshness is physical dir deletion + `SSR_CACHE_SECS` TTL.
9. **Migrations.** Create `migrations/NNN_desc.sql` **and** append `("NNN", include_str!("../../migrations/NNN_desc.sql"))` to the `MIGRATIONS` array in `src/db/migrate.rs` (a compile-test guards file/array parity). Each migration runs in its own transaction; write them idempotent-safe.
10. **DB connections.** Runtime path = `get_conn()` (fast-fail — do NOT retry pool-full `Timeout`, to avoid avalanche); startup path = `get_conn_for_startup()`. `statement_timeout` is injected globally via libpq options — don't add per-query timeouts.
11. **Markdown/HTML rendering** (`render_markdown_enhanced`): pulldown-cmark + syntect classed highlighting + TOC + heading anchors, CPU-bound → `spawn_blocking`. **Article HTML is rendered once at save time and stored in `posts.content_html`** — modifying syntaxes does not auto-refresh existing posts; rebuild via the `/admin/posts` "rebuild all" button (`rebuild_content_html`, batch size 500).
12. **Code highlighting** (`src/highlight.rs`): syntect `ClassedHTMLGenerator` emits CSS classes paired with `public/highlight.css`. To add/fix a language: edit `syntaxes/<Lang>.sublime-syntax` (the `expression` context's `include` order matters — multi-token rules before single-token ones), validate the YAML, add a test asserting CSS classes, run `cargo test --features server highlight_code_<lang> -- --nocapture`, and regenerate `public/highlight.css` only if a new scope type was added (`cargo run --features server --bin generate_highlight_css`). See `DEVELOPMENT.md` for the full guide.
13. **WebP**: the `image` crate's `"webp"` feature is **intentionally excluded** — all WebP encode/decode goes through zenwebp (`src/webp.rs`). Do NOT add it.
14. **JS libs** (`libs/`): pnpm workspace, TypeScript strict (target ES2020, `verbatimModuleSyntax` ⟹ use `import type`), Biome formatter (2-space, single quotes, semicolons, `trailingCommas: all`, line width 100), Vite 8 IIFE bundles written into `../../public/<dir>/`. `@yggdrasil/shared` is inlined into each IIFE — IIFEs cannot import each other at runtime. Use `make build-libs` or `make build-<name>` (`pnpm --filter`).
15. **Heavy `//!` module docs explain WHY.** Read a module's top doc comment before editing it. User-facing strings are predominantly Chinese.

## Workflow

- **每完成一个功能点立即提交**。Agent 自主判断提交时机——当一个逻辑完整的改动通过验证(编译通过 / 测试通过)后,无需等待用户指令,直接 `git add` + `git commit`。
- 提交粒度按"功能点"而非"文件":相关联的多文件改动合并为一个提交,不相关的改动拆成多个提交。
- 提交信息遵循现有风格:`type(scope): 简述`,正文(可选)说明动机与关键改动。常见 type:`feat` / `fix` / `docs` / `refactor` / `chore` / `perf`。
- 只在用户明确要求时才 `git push`。提交到本地即可,不主动推送。

## Testing & QA

- **Layout**: mostly inline `#[cfg(test)] mod tests` unit tests across `src/` (~40 modules) plus exactly one integration file `tests/post_detail_slug_rerun.rs` (a source-string guard asserting a Dioxus render-purity antipattern is absent).
- **Philosophy**: pure-function unit tests deliberately decoupled from DB/FS/cache. Inject dependencies (closures, temp dirs) instead of touching live state. **No test connects to live Postgres** — there is no test DB harness.
- **Feature gating**: server-touching tests use `#[cfg(all(test, feature = "server"))]`; pure-logic tests use plain `#[cfg(test)]`.
- **`serial_test`** serializes tests that mutate *process-global* state (moka cache singletons in `cache.rs`, env vars in `csrf`/`rate_limit`, in-process task maps in `progress.rs`, Docker in `infra/docker.rs`) — never DB rows.
- **Docker tests** auto-skip when the daemon is unavailable (`require_docker().await` → `None` → `eprintln!("skip: Docker daemon 不可用")`).
- **Run**: `make test` (== `cargo test --features server`; default features already enable `server`). For a single highlight test with visible output: `cargo test --features server highlight_code_<lang> -- --nocapture`.
- **Highlighting tests** (`src/highlight.rs`, ~30 tests) are the canonical example of the test philosophy; include compile-consistency tests (`custom_syntax_list_matches_directory`, `migrations_match_files_on_disk`) that keep embedded arrays in sync with their on-disk directories.
- **Coverage**: no formal coverage target; tests defend invariants and guard footguns (migration/syntax array parity, render-purity anti-patterns).

## CI

Gitea Actions (`.gitea/workflows/ci.yaml`): a `check` job (lint/typecheck) and a `build` job. Docker images are **not** pushed by CI — deployment is manual via `scripts/xun.fish` (build → scp → rolling-restart `app` only → verify `/healthz` + `/readyz`). CI dashboard: https://git.rua.plus/api/v1/repos/xfy/yggdrasil/actions/tasks.

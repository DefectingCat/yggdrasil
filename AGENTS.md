# AGENTS.md

## Workflow

- **每完成一个功能点立即提交**。Agent 自主判断提交时机——当一个逻辑完整的改动通过验证(编译通过 / 测试通过)后,无需等待用户指令,直接 `git add` + `git commit`。
- 提交粒度按"功能点"而非"文件":相关联的多文件改动合并为一个提交,不相关的改动拆成多个提交。
- 提交信息遵循现有风格:`type(scope): 简述`,正文(可选)说明动机与关键改动。常见 type:`feat` / `fix` / `docs` / `refactor` / `chore` / `perf`。
- 只在用户明确要求时才 `git push`。提交到本地即可,不主动推送。

## Development Commands

```bash
make dev           # tailwindcss watch + dx serve (needs PostgreSQL)
make build         # build-editor → highlight-css → tailwindcss → doc → dx build --release
make build-linux   # same as build but targets x86_64-unknown-linux-musl
make css           # one-shot CSS
make css-watch     # watch mode
make test          # cargo test + vitest (tiptap-editor + lightbox libs)
make doc           # cargo doc (ayu 主题) → 拷贝到 public/doc/，随 build 发布
make doc-open      # 同 doc，生成后自动用浏览器打开（本地预览，不拷贝）
make clean         # cargo clean + rm public/style.css
```

**Build order matters**: `make build` runs `build-editor` → `highlight-css` (`cargo run --bin generate_highlight_css`) → `tailwindcss --minify` → `doc` (`cargo doc` + 拷贝到 `public/doc/`) → `dx build --release`. Do not run `dx build --release` alone.

## Prerequisites

- Rust 1.95+ with `wasm32-unknown-unknown` target
- `dx` CLI (`cargo install dioxus-cli`)
- `tailwindcss` CLI v4 — install via `npm install -g @tailwindcss/cli` (v4 splits the CLI into its own package; the `tailwindcss` core package has no `bin`), or use the standalone binary
- PostgreSQL running locally

## Environment

Create `.env` (not committed):

```
DATABASE_URL=postgres://postgres:postgres@localhost:5432/yggdrasil
RUST_LOG=info
```

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
DB_POOL_SIZE=20             # database connection pool size
MIGRATE_STARTUP_TIMEOUT_SECS=30  # how long startup waits for PostgreSQL before giving up
STATEMENT_TIMEOUT_SECS=30   # per-query timeout; slow queries are canceled to protect the pool
SSR_CACHE_SECS=3600         # incremental SSR cache TTL
```

Session / security tuning:

```
COOKIE_SECURE=false         # set true/1/yes to add Secure flag to session cookie
TRUSTED_PROXY_COUNT=0       # number of reverse proxies in front of the app; used to extract real client IP from X-Forwarded-For
APP_BASE_URL=               # e.g. https://your-domain.example — trusted origin for CSRF checks on write requests; unset falls back to Host header + X-Forwarded-Proto
```

Run migrations before first dev server start:

```bash
./migrate.sh       # auto-creates DB, runs migrations/ in order
```

## Architecture: Conditional Compilation

Dioxus 0.7 fullstack project with **two independent gates** — the most common source of compilation errors.

| Gate | Applies to | Used for |
|------|-----------|----------|
| `#[cfg(feature = "server")]` | Server binary only | DB, env loading, background tasks, server function bodies, highlight, WebP, caching |
| `#[cfg(target_arch = "wasm32")]` | WASM frontend only | localStorage, DOM APIs, web_sys calls, theme detection |

**Critical**: Both default features (`web` + `server`) are enabled in `Cargo.toml`. The `dx` CLI handles feature selection during builds.

**Stub pattern**: `src/db/mod.rs` provides a `DummyPool` when `server` feature is disabled — do not remove.

**Server-only helpers**: `src/auth/password.rs`, `src/auth/session.rs`, `src/api/auth.rs`, `src/api/comments/helpers.rs`, and several model helper methods are gated with `#[cfg(feature = "server")]` because they are only called from server function bodies, which are stripped in WASM builds.

**Server-only dependencies**: Crates that are only used behind `#[cfg(feature = "server")]` (e.g., `argon2`, `uuid`, `regex`, `pulldown-cmark`, `rand`, `http`, `sha2`, `hex`, plus the pre-existing optional server stack) are declared as `optional = true` in `Cargo.toml` and enabled only through the `server` feature. They are not compiled into the WASM frontend.

## Dual API Architecture

The server exposes two distinct API patterns:

1. **Dioxus server functions** (`#[server(Name, "/api")]` in `src/api/`) — auto-routed, callable from both client and server Rust. Spread across `src/api/auth.rs`, `src/api/posts/`, `src/api/comments/`, and `src/api/settings.rs`.

2. **Axum routes** (registered in `src/main.rs`) — manual `axum::Router` for endpoints that don't fit the server-function model:
   - `POST /api/upload` — image upload (multipart, auth-required, rate-limited)
   - `GET /uploads/{*path}` — image serving with on-the-fly resize/rotate/convert (query params: `w`, `h`, `thumb`, `rotate`, `format`, `quality`)

## Server Module Structure

```
src/api/          — server functions + Axum handlers
  auth.rs         — login, register, session validation
  comments/       — comment CRUD + approval/spam/trash server functions
  markdown.rs     — Markdown→HTML rendering (pulldown-cmark + ammonia sanitization)
  image.rs        — image serving with processing pipeline + disk+memory cache
  upload.rs       — image upload, auto-converts to WebP
  rate_limit.rs   — governor-based rate limiting (5 tiers: strict/upload/image/comment/unknown)
  settings.rs     — site settings server functions (trash retention, etc.)
  slug.rs         — URL slug generation
  posts/          — CRUD server functions for blog posts
src/auth/         — password hashing (Argon2) + session token management
src/bin/          — generate_highlight_css (build-time CSS generation)
src/cache.rs      — moka future-based caches for posts, tags, stats
src/components/   — Dioxus UI components
src/db/           — PostgreSQL pool (deadpool-postgres, LazyLock global)
src/hooks/        — shared Dioxus hooks
src/models/       — Post, User, Tag data models
src/pages/        — route page components (frontend + admin)
src/tasks/        — background tokio tasks (session cleanup)
src/theme.rs      — light/dark theme with SSR cookie + WASM localStorage
src/webp.rs       — zenwebp encode/decode (image crate has no WebP)
```

## Tiptap Editor Subproject

Rich-text editor in `libs/tiptap-editor/`, built as an IIFE library exposing `window.TiptapEditor`.

- Output: `public/tiptap/`
- `make build` runs `npm ci --include=dev && npm run build` inside `libs/tiptap-editor`; the `build` script is `tsc --noEmit && vite build` (Vite 8 / Rolldown, type-check before bundle)
- Unit tests: `npm test` (Vitest 4 + happy-dom), covering `UploadCoordinator` counts/lifecycle, `UploadImageNodeView` rendering/callbacks, and `isValidUrl`
- `src/pages/admin/write.rs` initializes via `src/tiptap_bridge.rs` (wasm-bindgen bindings): injects `Closure` callbacks (`onUpdate`/`onReady`/`onUploadEvent`/`onImageUpload`) into `TiptapEditor.create`, holds the instance + closures in `EditorHandle` (Drop calls `destroy()`). No `js_sys::eval`, no `window` globals, no polling.

Do not edit `public/tiptap/` — they are build artifacts.

## Lightbox Subproject

Image lightbox (click-to-zoom) in `libs/lightbox/`, built as an IIFE library. Unlike the Tiptap editor, it is **not** wired through wasm-bindgen; the built `lightbox.js` is injected globally via `Dioxus.toml` (`script = ["/lightbox/lightbox.js"]`), and `src/components/post/post_content.rs` sets `window.__lightboxSelectors` (e.g. `['.post-content', '.entry-cover']`) before the script loads. The IIFE tail reads this config and self-initializes; `post_content.rs` also calls it directly as a fallback if the script was already loaded.

- Output: `public/lightbox/` (`lightbox.js`, `lightbox.css`, `lightbox.js.map`)
- `make build` runs `npm ci --include=dev && npm run build` inside `libs/lightbox`; the `build` script is `tsc --noEmit && vite build`
- Unit tests: `npm test` (Vitest, 23 tests), covering `geometry` math and `lightbox` rendering/lifecycle

Do not edit `public/lightbox/` — they are build artifacts.

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

## Testing

```bash
make test          # cargo test (Rust, 402 tests) + vitest (tiptap-editor 46 tests, lightbox 23 tests)
dx check          # Dioxus type-check (catches component/Router issues)
cargo clippy      # lint
```

Most tests use `#[cfg(all(test, feature = "server"))]` — they only run when the server feature is active (which is the default). No integration tests requiring a database connection. The two `libs/` subprojects (tiptap-editor, lightbox) run their own vitest suites.

## Image Processing Constraints

- The `image` crate is configured **without** WebP support (`default-features = false, features = ["jpeg", "png", "gif"]`). Do not add WebP to the image crate features.
- All WebP encode/decode goes through `zenwebp` via `src/webp.rs`.
- Upload pipeline auto-converts non-GIF/non-WebP images to WebP, keeping original format if WebP is larger.
- Image serving supports on-the-fly resize (`w`, `h`), thumbnail (`thumb=WxH`), rotation (90/180/270), and format conversion.

## Build Artifacts (gitignored)

- `public/style.css` — Tailwind output
- `public/highlight.css` — generated by `generate_highlight_css` binary
- `public/tiptap/` — Vite build output (editor)
- `public/lightbox/` — Vite build output (lightbox)
- `/dist`, `/.dioxus`, `/target`
- `node_modules` (inside `libs/tiptap-editor/` and `libs/lightbox/`)
- `uploads/.cache/` — image processing disk cache

## Notes

- `rand` is optional and only enabled by the `server` feature; it is not compiled into the WASM frontend.
- `#[allow(unused_mut, unused_variables)]` on `Write` component is intentional — `mut` signals are used in `#[cfg(target_arch = "wasm32")]` blocks stripped in server builds.
- Server uses incremental rendering with 300s cache (`IncrementalRendererConfig` in `src/main.rs`).

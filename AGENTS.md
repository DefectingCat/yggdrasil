# AGENTS.md

## Development Commands

```bash
make dev           # tailwindcss watch + dx serve (needs PostgreSQL)
make build         # build-editor → highlight-css → tailwindcss → dx build --release
make build-linux   # same as build but targets x86_64-unknown-linux-musl
make css           # one-shot CSS
make css-watch     # watch mode
make test          # cargo test
make clean         # cargo clean + rm public/style.css
```

**Build order matters**: `make build` runs `build-editor` → `highlight-css` (`cargo run --bin generate_highlight_css`) → `tailwindcss --minify` → `dx build --release`. Do not run `dx build --release` alone.

## Prerequisites

- Rust 1.95+ with `wasm32-unknown-unknown` target
- `dx` CLI (`cargo install dioxus-cli`)
- `tailwindcss` CLI v4 (standalone binary)
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
RATE_LIMIT_STRICT_PER_SEC=1
RATE_LIMIT_STRICT_BURST=5
RATE_LIMIT_UPLOAD_PER_SEC=2
RATE_LIMIT_UPLOAD_BURST=15
RATE_LIMIT_IMAGE_PER_SEC=10
RATE_LIMIT_IMAGE_BURST=50
DB_POOL_SIZE=20             # database connection pool size
SSR_CACHE_SECS=3600         # incremental SSR cache TTL
```

Session / security tuning:

```
COOKIE_SECURE=false         # set true/1/yes to add Secure flag to session cookie
TRUSTED_PROXY_COUNT=0       # number of reverse proxies in front of the app; used to extract real client IP from X-Forwarded-For
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

**Dead code allowances**: `src/auth/password.rs` and `src/auth/session.rs` carry `#[allow(dead_code)]` because the compiler sees them as unused in WASM builds where server function bodies are stripped.

## Dual API Architecture

The server exposes two distinct API patterns:

1. **Dioxus server functions** (`#[server(Name, "/api")]` in `src/api/`) — auto-routed, callable from both client and server Rust. Spread across `src/api/auth.rs` and `src/api/posts/`.

2. **Axum routes** (registered in `src/main.rs`) — manual `axum::Router` for endpoints that don't fit the server-function model:
   - `POST /api/upload` — image upload (multipart, auth-required, rate-limited)
   - `GET /uploads/{*path}` — image serving with on-the-fly resize/rotate/convert (query params: `w`, `h`, `thumb`, `rotate`, `format`, `quality`)

## Server Module Structure

```
src/api/          — server functions + Axum handlers
  auth.rs         — login, register, session validation
  markdown.rs     — Markdown→HTML rendering (pulldown-cmark + ammonia sanitization)
  image.rs        — image serving with processing pipeline + disk+memory cache
  upload.rs       — image upload, auto-converts to WebP
  rate_limit.rs   — governor-based rate limiting (3 tiers: strict/upload/image)
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
- `make build` runs `npm install && npx vite build` inside `libs/tiptap-editor`
- `src/pages/admin/write.rs` initializes via `js_sys::eval`, polls `window.__tiptap_ready`

Do not edit `public/tiptap/` — they are build artifacts.

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
cargo test        # standard Rust test suite
dx check          # Dioxus type-check (catches component/Router issues)
cargo clippy      # lint
```

Most tests use `#[cfg(all(test, feature = "server"))]` — they only run when the server feature is active (which is the default). No integration tests requiring a database connection.

## Image Processing Constraints

- The `image` crate is configured **without** WebP support (`default-features = false, features = ["jpeg", "png", "gif"]`). Do not add WebP to the image crate features.
- All WebP encode/decode goes through `zenwebp` via `src/webp.rs`.
- Upload pipeline auto-converts non-GIF/non-WebP images to WebP, keeping original format if WebP is larger.
- Image serving supports on-the-fly resize (`w`, `h`), thumbnail (`thumb=WxH`), rotation (90/180/270), and format conversion.

## Build Artifacts (gitignored)

- `public/style.css` — Tailwind output
- `public/highlight.css` — generated by `generate_highlight_css` binary
- `public/tiptap/` — Vite build output
- `/dist`, `/.dioxus`, `/target`
- `node_modules` (inside `libs/tiptap-editor/`)
- `uploads/.cache/` — image processing disk cache

## Notes

- `rand` + `getrandom` with `js` feature are required for Argon2 salt generation in WASM builds.
- `#[allow(unused_mut, unused_variables)]` on `Write` component is intentional — `mut` signals are used in `#[cfg(target_arch = "wasm32")]` blocks stripped in server builds.
- Server uses incremental rendering with 300s cache (`IncrementalRendererConfig` in `src/main.rs`).

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Yggdrasil is a Dioxus 0.7 fullstack web application — a blog system with user authentication. It compiles to both a WASM frontend and a native server backend from a single Rust codebase.

## Development Commands

```bash
# Development server (Tailwind CSS watch + dx serve)
make dev

# Production build (minified CSS + release binary)
make build

# Build CSS only
make css

# Watch CSS only
make css-watch

# Standard Rust commands
cargo build
cargo clippy
cargo test
cargo clean && make clean

# Dioxus CLI commands
dx serve                    # Dev server
dx build --release          # Release build
dx check                    # Type-check the project
```

## Prerequisites

- Rust 1.95+ with `wasm32-unknown-unknown` target
- `dx` CLI (Dioxus CLI) — install via `cargo install dioxus-cli`
- `tailwindcss` CLI v4 — standalone binary needed for `make dev`/`make build`
- PostgreSQL database running locally

## Environment Setup

Create a `.env` file with:

```
DATABASE_URL=postgres://postgres:postgres@localhost:5432/yggdrasil
```

Run the migration in `migrations/001_init.sql` to set up the `users` and `sessions` tables.

## Architecture

### Fullstack Dioxus

The project uses Dioxus's fullstack mode with two Cargo features:

- **`web`** (`dioxus/web`): Compiles the frontend to WASM. Activated by default.
- **`server`** (`dioxus/server`): Compiles the server-side code. Activated by default.

Server functions are defined with `#[server(Name, "/api")]` in `src/api/auth.rs`. These are callable from both client and server code. Dioxus handles the HTTP transport automatically.

### Conditional Compilation

Code is gated by two mechanisms:

- `#[cfg(feature = "server")]` — server-only code (database, background tasks, env loading). The `db` and `tasks` modules are entirely server-gated.
- `#[cfg(target_arch = "wasm32")]` — browser-only code (localStorage, cookie manipulation, DOM APIs).

The `db::pool` module provides a `DummyPool` stub when the `server` feature is disabled, so the code compiles for WASM.

### Authentication Flow

1. **Registration** (`src/api/auth.rs:register`): First registration creates the admin account. Subsequent registrations are rejected (`"Registration is closed"`). Passwords are hashed with Argon2.
2. **Login** (`src/api/auth.rs:login`): Validates credentials against the `users` table, creates a session record with a UUID token, returns the token.
3. **Session Storage**: The token is stored in a browser cookie (client-side, not HttpOnly). The server checks the `sessions` table for valid, non-expired tokens.
4. **Logout** (`src/api/auth.rs:logout`): Clears the client cookie and deletes expired sessions from the database.

### Database

- PostgreSQL via `tokio-postgres` with `deadpool-postgres` connection pooling.
- The pool is a `LazyLock` global in `src/db/pool.rs`, initialized from `DATABASE_URL`.
- Pool max size is 10 connections.

### Background Tasks

On server startup (`src/main.rs`), a background thread runs `tasks::session_cleanup::run_cleanup()`, which deletes expired sessions every hour.

### Routing

Routes are defined in `src/router.rs` using `#[derive(Routable)]`:

- `/` — Home (landing page with login/register links)
- `/login` — Login page
- `/register` — Registration page
- `/admin` — Admin dashboard (redirects to `/login` if not authenticated)

### Styling

- Tailwind CSS v4, compiled from `input.css` (`@import "tailwindcss"`) to `public/style.css`.
- Dark mode is implemented via the `dark:` variant with a `data-theme` attribute on the `<html>` element.
- The `ThemeToggle` component persists the preference in `localStorage`.

## Key Files

| File | Purpose |
|------|---------|
| `src/router.rs` | Route definitions and root app component with theme wrapper |
| `src/api/auth.rs` | Server functions: `register`, `login`, `logout`, `get_current_user` |
| `src/db/pool.rs` | Database connection pool (`LazyLock<Pool>`) |
| `src/auth/password.rs` | Argon2 password hashing and verification |
| `src/auth/session.rs` | UUID token generation and expiry calculation |
| `src/models/user.rs` | `User` struct and `UserRole` enum (`Admin` / `Blocked`) |
| `src/pages/admin.rs` | Admin dashboard with auth check and logout |
| `src/tasks/session_cleanup.rs` | Hourly background job to purge expired sessions |
| `src/theme.rs` | Dark/light theme state management and toggle button |
| `migrations/001_init.sql` | Database schema (users + sessions tables) |
| `Dioxus.toml` | Dioxus app config (default platform = web) |
| `Makefile` | Convenience targets for dev/build/css |

## Important Notes

- The session cookie is set client-side via `web_sys::HtmlDocument::set_cookie`, which means it is **not HttpOnly**. This is a known limitation of the current implementation.
- `get_current_user` returns the most recently created valid session (not necessarily the one matching the current request's cookie). The admin page relies on this for authentication state.
- The `#[allow(dead_code)]` attributes on auth utilities are needed because the compiler sees them as unused in WASM builds where server functions are stripped.
- `rand` with `getrandom` and `getrandom` with `js` feature are required for Argon2 salt generation in WASM builds.

# MCP Server Research — 2026 Best Practices for Embedding in Yggdrasil (Rust/Axum)

> Researched 2026-07-28 against primary sources only: the MCP specification at
> `modelcontextprotocol.io`, the `modelcontextprotocol` GitHub org (spec, rust-sdk),
> the Rust SDK (`rmcp`) source/crate metadata, RFCs, and client vendor docs.
> Every non-trivial claim carries a source link. Project code was not modified and
> no project build/test commands were run.

## TL;DR (read this first)

- **The current stable spec is `2025-11-25`** — NOT `2025-06-18` as our brief assumed.
  A development draft **`2026-07-28`** already exists and the Rust SDK tracks it.
  Source: [versioning page](https://modelcontextprotocol.io/docs/learn/versioning).
- **Streamable HTTP is the de-facto and only spec-blessed remote transport.** The old
  HTTP+SSE transport was removed after `2024-11-05`. Hypothesis CONFIRMED.
  Source: [transports spec](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports).
- **Stateless mode is now the SDK default** (draft `2026-07-28`, SEP-2567): no
  `Mcp-Session-Id`, no GET/DELETE stream. This fits a request-scoped Axum handler
  perfectly and removes the hardest part of the protocol (session lifecycle).
  Source: [rmcp README — Stateless Streamable HTTP](https://github.com/modelcontextprotocol/rust-sdk).
- **Authorization is OPTIONAL in the spec.** OAuth 2.1 is the heavy, spec-blessed path;
  for a **single-tenant blog where the admin mints tokens for their own clients**, a
  static bearer token in the `Authorization: Bearer` header is the pragmatic, widely
  supported choice. Claude Code, Cursor, and Cline all accept a custom `Authorization`
  header verbatim. Hypothesis CONFIRMED, with caveats (Origin validation is **MUST**).
  Source: [authorization spec](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization).
- **Use the official `rmcp` crate** (`v2.2.0` stable, `3.0.0-beta.2` dev, 3.4M
  downloads/month, 1,765 dependent crates, Apache-2.0). It ships a Tower
  `StreamableHttpService` that mounts on Axum with one line:
  `axum::Router::new().nest_service("/mcp", service)`. Do NOT hand-roll JSON-RPC.
  Source: [rmcp on lib.rs](https://lib.rs/crates/rmcp), [rmcp README](https://github.com/modelcontextprotocol/rust-sdk).
- **Knowledge-base pattern is confirmed:** expose PUBLISHED posts as **Resources**
  (URI-addressable, `resources/list` paginated, `resources/read`) AND ship a `search`
  **Tool**. Tools are model-invoked; Resources are app/agent-selected. Both are the
  documented split. Source: [resources](https://modelcontextprotocol.io/specification/2025-11-25/server/resources),
  [tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).

---

## 1. Protocol & Transports

### 1.1 Spec version & date

The **current** protocol version is **`2025-11-25`** (status *Current*). The version
identifier format is `YYYY-MM-DD`, bumped only on backwards-*incompatible* changes.
Source: [versioning page](https://modelcontextprotocol.io/docs/learn/versioning).

A **development draft `2026-07-28`** is already in flight; the `rmcp` README states it
"tracks the MCP `2026-07-28` draft while remaining fully compatible with the stable
`2025-11-25` release and earlier versions." Its headline features: server discovery &
negotiation, transport-neutral subscriptions, long-running tasks, response caching,
multi-round-trip requests, and standard HTTP routing headers.
Source: [rmcp README](https://github.com/modelcontextprotocol/rust-sdk).

### 1.2 Defined transports

The spec defines exactly **two** transports ([transports spec](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)):

1. **stdio** — client launches the server as a subprocess; messages over stdin/stdout.
   "Clients SHOULD support stdio whenever possible." (Local only.)
2. **Streamable HTTP** — server is an independent process; a **single MCP endpoint**
   (e.g. `https://example.com/mcp`) handles POST (send JSON-RPC) and optionally GET
   (open an SSE stream). The server MAY stream replies as `text/event-stream` or reply
   with a single `application/json` body. The client MUST send `Accept: application/json, text/event-stream`.

The **HTTP+SSE transport (2024-11-05) is removed/deprecated.** The spec states
Streamable HTTP "replaces the HTTP+SSE transport from protocol version 2024-11-05" and
provides a backwards-compatibility note for servers that want to keep serving old clients.
Source: [transports — Backwards Compatibility](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports#backwards-compatibility).

### 1.3 Recommendation for an internet-facing server in 2026

**Streamable HTTP is the de-facto standard for hosted/remote MCP.** It is the only
HTTP transport the current spec defines, and every major client (Claude Code/Desktop,
Cursor, Cline, Continue) supports it natively. stdio is explicitly local-only.
Source: [transports spec](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports);
client support: [Claude Code MCP](https://code.claude.com/docs/en/mcp).

### 1.4 What changed across revisions

**`2024-11-05` → `2025-03-26`:** introduced the HTTP+SSE transport (later replaced).
Authorization framework introduced at the transport layer.

**`2025-03-26` → `2025-06-18`** ([changelog](https://modelcontextprotocol.io/specification/2025-06-18/changelog)) — major:
1. Removed JSON-RPC **batching**.
2. Added **structured tool output** (`outputSchema`, `structuredContent`).
3. Reclassified MCP servers as **OAuth 2.1 Resource Servers** with Protected Resource
   Metadata discovery (RFC 9728).
4. **Required** clients to implement **Resource Indicators (RFC 8707)**.
5. Added the **Security Best Practices** page.
6. Added **Elicitation** (server asks client/user for input).
7. Added **Resource Links** in tool results.
8. **Required** the `MCP-Protocol-Version` header on all subsequent HTTP requests.
9. Changed lifecycle Operation SHOULD → **MUST**.

**`2025-06-18` → `2025-11-25`** ([changelog](https://modelcontextprotocol.io/specification/2025-11-25/changelog)) — major:
1. Added **OpenID Connect Discovery 1.0** support in AS discovery.
2. **Icons** metadata on tools/resources/prompts.
3. **Incremental scope consent** via `WWW-Authenticate` (SEP-835).
4. Tool-name guidance (SEP-986).
5. Richer **Elicitation** schemas (enums, single/multi-select, defaults) and **URL-mode elicitation**.
6. **Tool calling inside sampling** (`tools`/`toolChoice`).
7. **OAuth Client ID Metadata Documents** (SEP-991) as a recommended client-registration mechanism.
8. **Experimental Tasks** extension (SEP-1686) — durable, polled long-running requests.
9. JSON Schema **2020-12** as the default dialect.
10. Servers MUST respond **403** for invalid `Origin` headers in Streamable HTTP.

**`2025-11-25` → draft `2026-07-28`** (per rmcp README): server discovery/negotiation,
transport-neutral `subscriptions/listen` (replacing `resources/subscribe` + standalone GET),
Tasks extension, response caching (`ttlMs`/`cacheScope`), multi-round-trip requests
(MRTR, SEP-2322), standard HTTP routing headers (`Mcp-Method`/`Mcp-Name`/`Mcp-Param-*`,
SEP-2243), and **stateless Streamable HTTP by default (SEP-2567)**.

### 1.5 Session management — required vs optional vs stateless

Session handling is **optional, not required** ([transports — Session Management](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports#session-management)):

- A Streamable HTTP server **MAY** assign an `MCP-Session-Id` in the `InitializeResult`
  response. If present, the client MUST echo it on every subsequent request; a server
  that requires it SHOULD 400 requests lacking it.
- The session ID MUST be only visible ASCII (`0x21`–`0x7E`), SHOULD be cryptographically
  secure (UUID/JWT/hash).
- **Stateless mode** is explicitly supported: a server simply never returns a session ID
  and answers each POST independently. In draft `2026-07-28` (SEP-2567) this becomes the
  **default** for the modern protocol — "no `Mcp-Session-Id`, no standalone GET/DELETE
  stream, and no `Last-Event-ID` resumption." ([rmcp README](https://github.com/modelcontextprotocol/rust-sdk)).

The `MCP-Protocol-Version` header (e.g. `MCP-Protocol-Version: 2025-11-25`) is **MUST**
on every post-initialize HTTP request; for backwards compat a server without other info
SHOULD assume `2025-03-26`, and MUST 400 on an unsupported version.
Source: [transports — Protocol Version Header](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports#protocol-version-header).

> **Implication for Yggdrasil:** run **stateless**. No session store, no `Mcp-Session-Id`.
> Each POST is a self-contained Axum handler. This collapses the protocol to "JSON-RPC
> over POST, optionally streaming the reply."

---

## 2. Auth & Security

### 2.1 What the spec mandates for remote (HTTP) servers

Authorization is **OPTIONAL** ([authorization spec](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)):
> "Authorization is OPTIONAL for MCP implementations. When supported: Implementations
> using an HTTP-based transport SHOULD conform to this specification."

When you DO implement spec-conformant auth, it is **OAuth 2.1** layered on:
- **OAuth 2.1 IETF draft** (`draft-ietf-oauth-v2-1-13`), whose security BCP is
  [RFC 9700](https://datatracker.ietf.org/doc/html/rfc9700).
- **PKCE** (S256) — clients MUST implement it and MUST refuse to proceed without
  `code_challenge_methods_supported`.
- **Resource Indicators ([RFC 8707](https://www.rfc-editor.org/rfc/rfc8707.html))** —
  clients MUST send `resource=` in auth+token requests; servers MUST validate tokens
  were issued for them (audience binding).
- **OAuth 2.0 Protected Resource Metadata ([RFC 9728](https://datatracker.ietf.org/doc/html/rfc9728))**
  — servers MUST publish `authorization_servers` via a `WWW-Authenticate` 401 challenge
  and/or `.well-known/oauth-protected-resource`.
- **Authorization Server Metadata ([RFC 8414](https://datatracker.ietf.org/doc/html/rfc8414))**
  or **OpenID Connect Discovery 1.0** for AS discovery.
- **Dynamic Client Registration ([RFC 7591](https://datatracker.ietf.org/doc/html/rfc7591))** — MAY;
  now secondary to **Client ID Metadata Documents (SEP-991)**.

Crucially, the token always travels as a **bearer** in the standard header:
`Authorization: Bearer <access-token>` — MUST be on **every** HTTP request, MUST NOT be
in the query string. Source: [Access Token Usage](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization#access-token-usage).

### 2.2 The practical alternative for a single-tenant blog

Full OAuth 2.1 (PKCE + DCR + resource metadata + an authorization server you host) is
vastly over-engineered for **one admin minting tokens for their own AI clients
(Claude Desktop, Cursor, Cline)**. The spec leaves the door open by making auth OPTIONAL,
and **every major client supports a static bearer token via the `Authorization` header**,
which is exactly the `Authorization: Bearer` form the spec mandates for OAuth tokens
anyway:

- **Claude Code / Claude Desktop**: `claude mcp add --transport http <name> <url> --header "Authorization: Bearer <token>"`;
  JSON form uses `type: "http"` (alias `streamable-http`), `url`, `headers.Authorization`.
  Source: [Claude Code MCP docs](https://code.claude.com/docs/en/mcp).
- **Cursor**: `~/.cursor/mcp.json` → `type: "streamable-http"` (or `"http"`), `url`,
  `headers.Authorization`. Source: [Cursor MCP docs](https://docs.cursor.com/context/model-context-protocol).
- **Cline**: `cline_mcp_settings.json` → `type: "streamableHttp"`, `url`,
  `headers.Authorization`, plus `disabled`/`autoApprove`. ⚠️ Cline historically lets its
  OAuth-provider logic override custom `Authorization` headers on `sse` entries, so
  `streamableHttp` is the recommended type for bearer tokens. Source: [Cline docs / GitHub](https://cline.bot),
  [GitHub issue on header override](https://github.com/cline/cline).

**So: model the credential as an opaque bearer token, validate it per-request, and skip
the OAuth dance entirely.** The header format is identical to what an OAuth-issued token
would use, so this path is forward-compatible if you ever add real OAuth.

Caveats the spec forces regardless of token source:
- **Token passthrough is forbidden.** "MCP servers MUST NOT accept any tokens that were
  not explicitly issued for the MCP server." ([Token Passthrough](https://modelcontextprotocol.io/specification/2025-11-25/basic/security_best_practices#token-passthrough)).
  For a self-issued opaque token validated by SHA-256 lookup, this is automatically
  satisfied — the token *is* issued for our server.
- **Sessions MUST NOT be used for authentication.** "MCP servers that implement
  authorization MUST verify all inbound requests. MCP Servers MUST NOT use sessions for
  authentication." ([Session Hijacking](https://modelcontextprotocol.io/specification/2025-11-25/basic/security_best_practices#session-hijacking)).
  → The bearer token authenticates every request; `Mcp-Session-Id` (if ever used) is
  transport-only and untrusted.
- **Token theft guidance:** "Authorization servers SHOULD issue short-lived access
  tokens." → make the admin-selectable lifetime short-by-default and rotatable.
  Source: [Token Theft](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization#token-theft).
- **Scope minimization** is an explicit spec recommendation: avoid wildcard/omnibus
  scopes, prefer progressive least-privilege. Source: [Scope Minimization](https://modelcontextprotocol.io/specification/2025-11-25/basic/security_best_practices#scope-minimization).

### 2.3 The 2025 security incidents & mitigations

- **Tool Poisoning Attack (TPA)** — coined/disclosed by **Invariant Labs** (April 2025;
  since acquired by Snyk). An attacker (often a malicious or compromised MCP server)
  embeds hidden instructions in tool *descriptions*/*schemas*; the LLM treats them as
  authoritative and may exfiltrate data or call other tools. This is a form of indirect
  prompt injection that turns the agent into a **confused deputy**. Mitigations:
  treat tool metadata and tool *output* as untrusted; human-in-the-loop approval for
  mutating calls; narrow scopes. Source: [Invariant Labs — Tool Poisoning Attacks](https://invariantlabs.ai/blogs/tool-poisoning-attacks/).
- **Confused-deputy / over-privileged proxy** — the spec's canonical example: an MCP
  proxy holding broad OAuth tokens to a third-party API is tricked into misusing them.
  Mitigations: per-client consent, exact `redirect_uri` matching, per-request auth,
  narrow scopes. Source: [Confused Deputy Problem](https://modelcontextprotocol.io/specification/2025-11-25/basic/security_best_practices#confused-deputy-problem).
- **Rug.md / malicious resources** — the broader supply-chain risk that a benign-looking
  resource (e.g. a markdown file) carries injection payloads. Mitigation: read paths
  (Resources) should not be auto-executed; segregate read-only from mutating Tools.
- **Token Passthrough** — covered above; explicitly forbidden.
- **SSRF via OAuth metadata URLs** — clients fetching `WWW-Authenticate`/resource-metadata
  URLs must block private IPs / cloud metadata. (Relevant if Yggdrasil ever acts as an
  MCP *client*; less so as a server.) Source: [SSRF](https://modelcontextprotocol.io/specification/2025-11-25/basic/security_best_practices#server-side-request-forgery-ssrf).
- **Session hijacking** — covered above.
- **Lethal trifecta** (community framing, Simon Willison et al.): private-data access +
  untrusted content + outbound capability = trivial exfiltration. Mitigation: don't give
  a single token all three; scope tools down.
- **`mcp-remote` CVE-2025-6514** — a command-injection bug in the popular `mcp-remote`
  shim that bridges stdio clients to remote servers. Lesson: vet the transport shim, not
  just your server.

### 2.4 Mandatory server-side hardening (from the Streamable HTTP security warning)

The spec's Streamable HTTP section makes three requirements/non-recommendations
([Security Warning](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports#streamable-http)):

1. Servers **MUST validate the `Origin` header** on all connections to prevent DNS
   rebinding; invalid Origin → **403**. *(From 2025-11-25 this 403 is a hard MUST.)*
2. Local servers SHOULD bind to localhost (not relevant — we are remote).
3. Servers **SHOULD implement proper authentication for all connections.**

---

## 3. Primitives to Implement

MCP primitives split into **server-offered** (Resources, Prompts, Tools, Logging,
Completions) and **client-offered** (Sampling, Roots, Elicitation). For a content/CMS
MCP server the relevant set is small. Sources:
[lifecycle capabilities](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle),
[resources](https://modelcontextprotocol.io/specification/2025-11-25/server/resources),
[tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).

| Primitive | Purpose | Yggdrasil? |
|---|---|---|
| **Tools** | Model-invoked functions (`tools/call`). Model-controlled. | **Yes** — search, create/update/publish posts, moderate comments, manage tags/settings. |
| **Resources** | App/agent-selected context, addressable by **URI**, listable via `resources/list`, readable via `resources/read`. | **Yes** — each PUBLISHED post = one Resource. |
| **Resource Templates** | URI templates (RFC 6570) for parameterized resources. | **Yes** — e.g. `post://{slug}` or `https://rua.plus/posts/{slug}`. |
| **Prompts** | Templated user-facing workflows (`prompts/get`). | Optional — e.g. a "draft a post about X" prompt. Low priority. |
| **Completions** | Argument autocompletion. | Optional, skip for MVP. |
| **Logging** | Structured log stream to client. | Optional; **deprecated** in draft `2026-07-28` (SEP-2577). Skip. |
| **Elicitation** | Server asks client/user for structured input mid-operation. | Optional; useful for confirmations but adds complexity. Skip for MVP. |
| **Sampling** | Server asks client's LLM to run a completion. | **Skip.** Deprecated (SEP-2577); not needed. |
| **Roots** | Client tells server its workspace boundaries. | **Skip** (local-IDE concept). Deprecated (SEP-2577). |
| **Tasks** | Long-running, polled operations (2025-11-25 experimental). | Skip for MVP; only relevant for very slow ops. |

### 3.1 The knowledge-base pattern (Resources + a search Tool) — recommended

This **is** the documented split and the recommended pattern:

- **Resources are "application-driven"** — the host decides how to incorporate them
  (UI picker, heuristics, or the model's selection). Each is identified by a URI and
  returns `text` or base64 `blob` content. Source: [resources — User Interaction Model](https://modelcontextprotocol.io/specification/2025-11-25/server/resources).
- **Tools are "model-controlled"** — the LLM discovers and invokes them automatically.
  "There SHOULD always be a human in the loop with the ability to deny tool invocations."
  Source: [tools — User Interaction Model](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).

Concretely for Yggdrasil:
- `resources/list` → paginated list of PUBLISHED posts (title, URI, mimeType `text/markdown`).
- `resources/templates/list` → `post://{slug}` (or the canonical HTTPS URL).
- `resources/read` → full rendered Markdown/HTML of one post.
- A `search_posts` **Tool** → keyword/FTS query, returns summaries + `resource_link`s
  to the matching posts (the `resource_link` content type, added 2025-06-18, lets a tool
  return URIs the client can then `resources/read`). Source: [Resource Links](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#resource-links).
- Mutating Tools (`create_post`, `update_post`, `publish_post`, `delete_post`,
  `moderate_comment`, `update_settings`, `manage_tags`) for admin tokens.

### 3.2 Pagination of large resource lists

MCP uses **opaque cursor-based pagination**, not numbered pages
([pagination spec](https://modelcontextprotocol.io/specification/2025-11-25/server/utilities/pagination)):

- Server returns `nextCursor` when more exist; client passes `cursor` to continue.
- **Page size is server-determined**; clients MUST NOT assume a fixed size.
- Applies to `resources/list`, `resources/templates/list`, `prompts/list`, `tools/list`.
- Clients MUST treat cursors as opaque; invalid cursor → error `-32602` (Invalid params).
- "Pagination is especially important when connecting to external services over the
  internet."

> Implement `resources/list` with a cursor (e.g. base64 of `("published_at", id)` or a
> signed offset). For the search Tool, cap result count and return `resource_link`s
> rather than dumping entire bodies.

---

## 4. Client Discovery / Config

All major clients consume a **`mcpServers` JSON object** keyed by server name. For a
remote Streamable HTTP server with a bearer token the entries are structurally identical
(`url` + `type` + `headers.Authorization`), differing only in the exact `type` spelling
and the file location. **A "Generate MCP config" admin feature should emit one block per
client** (or a canonical block plus copy-paste variants).

### Minimal example (canonical — works for Claude Code & Cursor)

```json
{
  "mcpServers": {
    "yggdrasil": {
      "type": "streamable-http",
      "url": "https://rua.plus/mcp",
      "headers": {
        "Authorization": "Bearer ygg_<opaque-token>"
      }
    }
  }
}
```

### Per-client specifics

| Client | File | `type` value | Notes | Source |
|---|---|---|---|---|
| **Claude Code / Desktop** | `.mcp.json` (project) or `~/.claude.json` (user) | `"http"` (alias `"streamable-http"`) | `type` is **required** when `url` present or the entry is misread as stdio. CLI: `claude mcp add --transport http <name> <url> --header "Authorization: Bearer …"`. | [Claude Code MCP](https://code.claude.com/docs/en/mcp) |
| **Cursor** | `~/.cursor/mcp.json` | `"streamable-http"` or `"http"` | Single `/mcp` endpoint; headers sent on every request. | [Cursor MCP](https://docs.cursor.com/context/model-context-protocol) |
| **Cline** | `…/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json` | `"streamableHttp"` | ⚠️ Use `streamableHttp`, **not** `sse` — Cline's OAuth logic can override custom `Authorization` headers on `sse` entries. Also has `disabled`, `autoApprove`. | [Cline docs](https://cline.bot) |
| **Continue** | `~/.continue/config.json` `experimental.mcpServers` | `"streamableHttp"` | Same `url`+`headers` shape. | Continue docs |

> The `Authorization: Bearer …` header shape is **identical** to what an OAuth-issued
> token would use ([Access Token Usage](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization#access-token-usage)),
> so a generated config is forward-compatible with a future OAuth flow.

A "Generate MCP config" button should:
1. Mint (or reuse) a token with the chosen scope + lifetime.
2. Render **three** ready-to-paste JSON snippets (Claude/Cursor share one; Cline uses
   `streamableHttp`; include the CLI one-liner for `claude mcp add`).
3. Warn that the snippet contains a secret and must not be committed.

---

## 5. Rust Ecosystem

### 5.1 The official SDK: `rmcp`

`rmcp` (crate `rmcp`, `modelcontextprotocol/rust-sdk`) is the **official** Rust MCP SDK,
tokio-based, Apache-2.0. As of 2026-07-28:

- **Latest stable: `2.2.0`** (2026-07-08); **dev: `3.0.0-beta.2`** (2026-07-24). There
  was a `0.1.x → 1.x` (migration guide discussion #716) and now a `1.x → 2.x` line.
- **3,398,090 downloads/month**, **1,765 dependent crates** (1,430 direct), Rust 2024
  edition. This is a mature, heavily-used crate, not a toy.
- Tracks `2026-07-28` draft while remaining compatible with `2025-11-25` and earlier.

Source: [rmcp on lib.rs](https://lib.rs/crates/rmcp), [rmcp README](https://github.com/modelcontextprotocol/rust-sdk).

### 5.2 Feature flags (cargo)

```toml
rmcp = { version = "2.2", features = [
  "server",                                  # ServerHandler + tool system (default)
  "macros",                                  # #[tool]/#[prompt] (default)
  "schemars",                                # JSON Schema gen for inputSchema/outputSchema
  "transport-streamable-http-server",        # StreamableHttpService (Tower)
  "reqwest",                                 # rustls TLS backend (if you also build a client)
] }
```
`auth` (OAuth) and `elicitation` are opt-in but **not needed** for a bearer-token server.
Source: [rmcp feature flags](https://lib.rs/crates/rmcp).

### 5.3 Mounting on Axum — there IS a first-class adapter

`StreamableHttpService` is a **Tower `Service`**, mounted on Axum with `nest_service`:

```rust
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, StreamableHttpServerConfig,
    session::local::LocalSessionManager,
};

let config = StreamableHttpServerConfig::default()
    .with_legacy_session_mode(false) // stateless for legacy versions too (SEP-2567)
    .with_json_response(true);       // plain application/json for simple tools,
                                     // falls back to text/event-stream when streaming needed

let service = StreamableHttpService::new(
    || Ok(YggMcpServer::new(state.clone())), // fresh handler per request (stateless!)
    LocalSessionManager::default().into(),
    config,
);

let router = axum::Router::new()
    .nest_service("/mcp", service)
    // your existing Axum routes merge here in src/main.rs
    .merge(existing_app_router());
```
Source: [rmcp README — Stateless Streamable HTTP](https://github.com/modelcontextprotocol/rust-sdk).

> Because there is no per-session state, the `service_factory` runs **per request**.
> Keep shared state (DB pool, moka cache) in a `Clone` handle captured by the closure;
> do not rely on in-memory state surviving between requests.

### 5.4 Defining tools/resources — declarative macros + trait

- `#[tool_router]` + `#[tool]` macros generate `inputSchema`/`outputSchema` from
  `schemars::JsonSchema` derive structs; handlers are plain `async fn`s returning
  `String`/`Vec<ContentBlock>`/`CallToolResult`/`Result<_, McpError>`.
- Resources: implement `list_resources`, `read_resource`, `list_resource_templates` on
  the `ServerHandler` trait; advertise the `resources` capability in `get_info()`.
- `list_changed` notifications let you push tool/resource-list updates without reconnect.
Source: [rmcp README — Tools / Resources](https://github.com/modelcontextprotocol/rust-sdk).

### 5.5 OAuth support (if ever needed)

`rmcp` has a full OAuth 2.1 *client* implementation (`auth` feature): PKCE-S256, RFC 8707
resource binding, RFC 9728/8414 discovery, DCR, Client ID Metadata Documents, scope
upgrade on 403, automatic token refresh, injectable HTTP client. A server-side OAuth
example (`servers_complex_auth_streamhttp`) exists. For our bearer-token design we do
**not** enable this. Source: [rmcp OAUTH_SUPPORT.md](https://github.com/modelcontextprotocol/rust-sdk/blob/main/docs/OAUTH_SUPPORT.md).

### 5.6 Hand-rolling JSON-RPC over Axum — feasibility & pitfalls

Feasible (it's JSON-RPC 2.0 over POST) but **not recommended**:
- You must implement the full lifecycle: `initialize` ↔ `notifications/initialized`,
  capability negotiation, version header validation, `MCP-Protocol-Version` handling,
  the SSE-vs-JSON response selection, `tools/list`+`tools/call`+`resources/list`+
  `resources/read`+`resources/templates/list`, pagination cursors, `list_changed`,
  error-code mapping (`-32602`, etc.).
- Pitfalls: getting the `Accept`/`Content-Type` negotiation wrong, forgetting the
  `Origin` 403, mishandling `MCP-Protocol-Version` (clients send the negotiated version),
  JSON-RPC batching being removed (don't accept arrays), and drift as the spec moves to
  `2026-07-28` (caching headers, MRTR, routing headers).
- `rmcp` already solved all of this and is a `tower::Service` — there is no reason to
  reimplement. The only case for hand-rolling is an exotic constraint (e.g. zero extra
  deps on `FROM scratch`), which does not apply to Yggdrasil's Axum binary.

---

## 6. Knowledge Base / Retrieval

### 6.1 MCP defines no embedding/semantic convention

There is **no** MCP "search" or "retrieval" primitive and no embedding convention. Search
is just a **Tool** whose `inputSchema` defines the query and whose result is content
(possibly `resource_link`s). Resources are the addressable corpus. This is intentional —
the protocol is agnostic to *how* you retrieve. Source: [tools spec](https://modelcontextprotocol.io/specification/2025-11-25/server/tools),
[resources spec](https://modelcontextprotocol.io/specification/2025-11-25/server/resources).

### 6.2 Full-text vs semantic/vector — tradeoffs

- **Postgres FTS (tsvector/tsquery, `websearch_to_tsquery`)** — already in Yggdrasil
  (`src/api/posts/search.rs`), zero new infra, ranking via `ts_rank`, good for a blog's
  vocabulary, deterministic, no embedding model to host/version. **MVP choice.**
- **pgvector + embeddings** — better recall for paraphrase/semantic queries, but adds:
  an embedding model (API call or local), a pipeline to (re)embed on publish/edit,
  a vector index (HNSW/IVFFlat), cost/latency per embed, and drift when the model is
  upgraded. Worth it only when FTS recall is demonstrably insufficient.

### 6.3 Recommended MVP for Yggdrasil

1. **Read path = Resources.** Expose every PUBLISHED post as a Resource (URI =
   canonical HTTPS URL or `post://{slug}`), `resources/list` paginated, `resources/read`
   returns rendered Markdown (cheaper for the model than raw HTML; respects the existing
   `content_html` cache by re-rendering).
2. **Search path = a `search_posts` Tool** backed by the existing Postgres FTS. Returns
   top-N `{title, slug, uri, snippet, published_at}` plus a `resource_link` for each so
   the client can pull full text via `resources/read`. This matches the [Resource Links](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#resource-links)
   pattern added in 2025-06-18.
3. **Defer vector retrieval** until FTS recall is measured and found wanting. The Tool
   boundary means swapping the backend later is invisible to clients.

> Note: a search Tool result is model-consumed text and is therefore an indirect-prompt-
> injection surface (attacker-controlled post bodies). Return snippets, validate/sanitize,
> and keep write Tools separately scoped. See §2.3.

---

## 7. Operational Hardening

### 7.1 Rate limiting keyed by token, not IP

- The spec's **Scope Minimization** and Token Theft guidance both push toward per-principal
  accountability. The existing Yggdrasil governor is keyed by **client IP**; for the MCP
  route the limiter MUST be keyed by the **bearer token** (or its hash), because (a) all
  requests hit one endpoint and IP-keying collapses many users behind a NAT/proxy, and
  (b) the token is the real principal.
- Implementation: an Axum `from_fn` middleware on `/mcp` that extracts the
  `Authorization` header, resolves the token to a `(user_id, scope)` via SHA-256 lookup,
  and feeds a `governor`/`tower` limiter keyed on `user_id` (or token id). The IP-keyed
  governor stays on the rest of the app.
- Bound per-request size, add a hard per-tool timeout (the spec recommends configurable
  per-request timeouts and honoring progress notifications — [Lifecycle — Timeouts](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle#timeouts)).

### 7.2 Token storage — hashing

- Store only the **SHA-256** of the token at rest (the spec says session IDs MAY be a
  "cryptographic hash"; the same principle applies to bearer tokens). On each request,
  `sha256(bearer)` → DB lookup → `(user_id, scopes, expires_at, revoked)`. This matches
  the brief's "sessions here are SHA-256 hashed."
- Tokens are long random opaque strings (≥32 bytes from a CSPRNG), prefixed (e.g.
  `ygg_…`) for searchability/leak-detection, with an admin-settable **lifetime**
  (default short, per the spec's "short-lived access tokens" guidance) and one-click
  **rotation/revocation**.

### 7.3 Per-token scopes (read / write / admin)

- Implement a coarse, least-privilege scope set (no wildcards — the spec explicitly warns
  against omnibus scopes). Suggested: `posts:read` (default), `posts:write`,
  `comments:moderate`, `settings:write`, `admin`. A token carries a subset.
- Map scopes to Tools at dispatch: `search_posts`/`resources/*` need `posts:read`;
  `create_post`/`update_post` need `posts:write`; `publish_post`/`delete_post` and
  `update_settings` need `admin`. Return HTTP 403 `insufficient_scope` (the spec's
  step-up pattern) when a tool is called without the scope — this is exactly the
  [WWW-Authenticate scope challenge](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization#scope-challenge-handling)
  shape, even though we are not doing full OAuth.
- Because the token *list* can be advertised per-token, an admin can mint a read-only
  token for a retrieval-only client and a full token for an authoring client.

### 7.4 Origin allowlisting & transport security

- **MUST validate `Origin`** on `/mcp` (DNS-rebinding defense) → 403 otherwise. Allowlist
  the configured site origin(s) and reject the rest. ([transports security warning](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports#streamable-http))
- Serve over HTTPS only (the nginx-proxy in front of Yggdrasil already terminates TLS).
  OAuth 2.1 requires HTTPS for all endpoints ([Communication Security](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization#communication-security));
  bearer tokens over plaintext are trivially sniffable, so enforce HTTPS at the proxy.

### 7.5 Audit logging

- Log every `tools/call` (and mutating `resources` ops if any) with token id, user,
  scope, tool name, args-hash, outcome, latency. The spec's "Accountability and Audit
  Trail" argument against token passthrough is exactly why per-token identity matters.
  Source: [Token Passthrough — Risks](https://modelcontextprotocol.io/specification/2025-11-25/basic/security_best_practices#token-passthrough).

### 7.6 Integration with the existing Axum stack

- Mount the `rmcp` `StreamableHttpService` at `/mcp` and merge it with the existing
  manual + Dioxus server-function router in `src/main.rs` exactly as other routes are.
- Layer the middleware **only on `/mcp`**: `bearer-auth → rate-limit(token) → origin-check → mcp-service`.
  The IP-keyed governor and cookie-session auth on the rest of the app are untouched.
- DB access reuses the existing `deadpool` pool; caching reuses `moka`; the existing FTS
  query in `src/api/posts/search.rs` backs the `search_posts` tool.

---

## Recommendation for Yggdrasil

A concrete, opinionated blueprint to turn into a spec.

### Transport
- **Streamable HTTP, single endpoint `/mcp`, stateless.** Pin `rmcp` and negotiate
  `2025-11-25`; when stable, opt into `2026-07-28` for free stateless-by-default + caching
  headers. No `Mcp-Session-Id`, no GET stream, no `Last-Event-ID`. Use
  `with_json_response(true)` (plain `application/json`) with SSE fallback only when a
  handler streams notifications.

### Auth model
- **Opaque bearer tokens** in `Authorization: Bearer ygg_<…>`, validated per-request by
  `sha256(token)` → DB lookup. **No OAuth 2.1 / PKCE / DCR** (overkill for single-tenant).
  The header format is identical to an OAuth bearer, so this is forward-compatible.
- Tokens are admin-generated in a settings page, **SHA-256 hashed at rest**, with a
  **user-selectable lifetime** (default e.g. 90 days, options 1/7/30/90/365 days +
  never-expire), one-click **rotate** and **revoke**.
- **Origin allowlist + HTTPS enforced** at nginx-proxy; 403 on bad Origin (spec MUST).

### Primitives
- **Resources:** every PUBLISHED post → one Resource (URI = canonical HTTPS URL).
  `resources/list` (paginated, opaque cursor), `resources/templates/list`
  (`post://{slug}`), `resources/read` (rendered Markdown). `listChanged` on publish/edit.
- **Tools (read):** `search_posts` (FTS-backed, returns summaries + `resource_link`s),
  `get_post`, `list_tags`.
- **Tools (write, scope-gated):** `create_post`, `update_post`, `publish_post`,
  `delete_post`, `moderate_comment`/`approve_comment`/`delete_comment`, `update_settings`,
  `create_tag`/`rename_tag`.
- Skip Prompts/Completions/Elicitation/Sampling/Roots/Tasks/Logging for MVP.

### SDK choice
- **`rmcp` v2.2.x** with features `server, macros, schemars, transport-streamable-http-server`.
  Mount `StreamableHttpService` via `axum::Router::nest_service("/mcp", service)` and merge
  with the existing router in `src/main.rs`. Do not hand-roll JSON-RPC.

### Scopes
- `posts:read` (default on every token), `posts:write`, `comments:moderate`,
  `settings:write`, `admin`. Map tool→scope at dispatch; return `403 insufficient_scope`
  on mismatch. Mint read-only tokens for retrieval clients.

### Rate-limit strategy
- New **token-keyed** limiter on `/mcp` only (extract bearer → `user_id` → governor key).
  Leave the existing IP-keyed governor on the web app. Add per-request body-size cap and
  per-tool timeout (honor progress notifications to extend).

### Client-config output format
"Generate MCP config" emits three snippets + a CLI one-liner, all pointing at
`https://rua.plus/mcp` with the minted `Authorization: Bearer`:

```json
// Claude Code (.mcp.json / ~/.claude.json) and Cursor (~/.cursor/mcp.json)
{
  "mcpServers": {
    "yggdrasil": {
      "type": "streamable-http",
      "url": "https://rua.plus/mcp",
      "headers": { "Authorization": "Bearer ygg_<token>" }
    }
  }
}
```
```json
// Cline (cline_mcp_settings.json) — MUST use streamableHttp, not sse
{
  "mcpServers": {
    "yggdrasil": {
      "type": "streamableHttp",
      "url": "https://rua.plus/mcp",
      "headers": { "Authorization": "Bearer ygg_<token>" },
      "disabled": false, "autoApprove": []
    }
  }
}
```
```bash
# Claude Code CLI
claude mcp add --transport http yggdrasil https://rua.plus/mcp \
  --header "Authorization: Bearer ygg_<token>"
```

### Where our hypothesis was right vs. needs amendment
- **Right:** Streamable HTTP is the correct (and only) remote transport; bearer tokens in
  the `Authorization` header are natively supported by every target client and are the
  pragmatic auth for a single-tenant blog; embed in the Axum binary via `rmcp`'s Tower
  service; expose published posts as Resources + a search Tool.
- **Amend (1):** target **`2025-11-25`**, not `2025-06-18` (and `2026-07-28` is imminent);
  the `MCP-Protocol-Version` header handling and `Origin`→403 rule are new MUSTs.
- **Amend (2):** run **stateless** (SEP-2567) — drop any plan for an `Mcp-Session-Id`
  store; sessions are transport-only and MUST NOT be used for auth.
- **Amend (3):** the rate limiter must be **token-keyed**, not the existing IP-keyed
  governor, and the mutating tools must be **scope-gated** (read/write/admin), not a
  single all-powerful admin token.
- **Amend (4):** treat search/tool output as an **indirect-prompt-injection surface**
  (Tool Poisoning / Rug.md) — sanitize snippets, keep read vs write tools on separate
  scopes, and rely on the client's human-in-the-loop for mutating calls.

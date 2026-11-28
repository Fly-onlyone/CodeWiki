---
tags: [core, cw-server]
---

# cw-server

> HTTP API of the CodeWiki backend.

## What it does

cw-server is the HTTP API of the CodeWiki backend. It serves data only, with no web frontend. The app, the MCP bridge and git hosts talk to it. Every route except `/v1/health` needs `Authorization: Bearer <token>`. `/v1/hooks/git` is signed with the webhook secret instead, because a git host cannot send the token.

`POST /v1/shutdown` stops the server; the app calls it on quit and before an update replaces the programs. `GET /v1/status` reports running updates, the last bridge call, waiting proposals per repository, and `notes_rev`. That counter moves when a vault or a claim changes, so clients know when to list waiting notes again.

## Source

- `crates/cw-server/Cargo.toml` — package manifest
- `crates/cw-server/src/lib.rs` — crate root

## How it works

`serve` binds the address, tells the caller the bound port through `on_bound` (useful for port 0), and builds the shared state with `state`. It then points semantic search at the embedder named in the settings and sets a one second inline embedding budget. On a blocking task it takes the write lock and tidies the repository list with `mark_worktrees` and `prune_worktrees`, then starts `embed_job::resume`. At last it runs axum with `api::routes`.

The stop future ends on the caller's `shutdown` future or on the `stop` notify of `POST /v1/shutdown`. It also closes the embedding job, so a background job does not keep the process alive. Running requests finish first.

`router` builds the same router without a socket, for tests. `ServerConfig` holds the data folder, token, optional webhook secret, whether settings are writable, an environment reader and a `Host`. Tests replace `Host` and the reader, so no model server is called. See [[Api Module]] for routes and [[Inner]] for shared state.

## Items

Modules:

- [[Activity Module]] — what the backend is doing right now, for `GET /v1/status`.
- [[Api Module]] — routes and handlers.
- [[Claims Module]] — which waiting notes an agent holds.
- [[Embed Job Module]] — the background embedding job: the one long job that may run.
- [[Pending Module]] — how many proposals wait in each repository, for `GET /v1/status`.
- [[Repos Module]] — the list of repositories this backend manages, stored as `repos.json`.
- [[Settings Module (cw-server)]] — what the settings routes do: describe the settings in effect, and change the saved ones.
- [[Workspace Module (cw-server)]] — views across every repository the backend manages.

Defined at the crate root:

- `pub struct ServerConfig` — public struct; used by `Inner`, `router`, `run`, `serve` and 1 more.
- `pub struct Host` — what the background embedding job asks of the machine. A test replaces it with a stand-in, so that it calls no model server.
- `const INLINE_BUDGET: Duration` — how long each step of an update of a wiki embeds for before it leaves the rest to the background. The embedder that needs no model is through thousands of documents in that time; a neural one gets a batch or two in, so that the first search after an update finds something by meaning.
- `pub const WEBHOOK_SECRET_ENV: &str` — environment variable that holds the webhook secret.
- [[Inner]] — struct
- [[AppState]] — struct
- `fn state(config: ServerConfig) -> AppState` — private function, returns `AppState`; called by `router`, `serve`.
- `pub fn router(config: ServerConfig) -> Router` — build the router. Exposed so tests can drive it without a socket.
- `serve` — serve until `shutdown` resolves or a client calls `POST /v1/shutdown`. Requests already running are finished first. Returns the bound address through `on_bound` so callers can publish the port when asking for port 0.

## Depends on

- [[Semantic Module]] — uses `behind`, `configure`, `embedder`, `set_inline_budget`
- [[cw-core]] — uses `CLIENT_HEADER`, `MCP_CLIENT`, `VERSION`
- [[Jobs (cw-server)]] — uses `close`, `default`, `status`
- [[Repos Module]] — uses `mark_worktrees`, `prune_worktrees`
- [[Service Module]] — uses `DOCUMENT_KINDS`, `open_existing_index_in`
- [[Error]] — uses `external`, `invalid`
- [[LocalEmbedder]] — uses `LocalEmbedder`, `default`
- [[Error Module]] — uses `Result`

## Used by

- [[Serve Module]] — from `run`
- [[Inner]] — from `Inner`

## Gotchas

- `settings_writable` is false for a server that listens beyond this machine. Otherwise a token holder could point a provider address at their own host and be sent the API key. The embedding job routes are closed for the same reason.
- With no webhook secret the git hook route answers 404.
- Embedding inside an update is limited to about one second. The rest runs in the background, so the first search after an update may find fewer results by meaning.
- Startup holds the write lock while it tidies the repository list, so a request that writes can wait a moment right after start.

## See also

- [[_index]]
- [[Crate Dependency Flow]]

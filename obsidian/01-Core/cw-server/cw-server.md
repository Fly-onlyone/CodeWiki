---
tags: [core, cw-server]
---

# cw-server

> HTTP API of the CodeWiki backend.

## What it does

The server exposes data only; it serves no web frontend. Every route except `/v1/health` requires `Authorization: Bearer <token>`. `/v1/hooks/git` is the other exception: a git host cannot send the token, so it signs its request with the webhook secret instead.

`POST /v1/shutdown` stops the server. The desktop app calls it when the user quits it, and before an update replaces the programs, so the old backend does not keep serving.

`GET /v1/status` says what the backend is doing: the wiki updates that are running and when an agent's MCP bridge last called (`activity`). Its `notes_rev` is a counter that moves whenever a vault or a claim on a waiting note changes, so a client knows when to list the waiting notes again.

## Source

- `crates/cw-server/Cargo.toml` — package manifest
- `crates/cw-server/src/lib.rs` — crate root

## How it works

Builds a library.

1. `run` in [[Serve Module]] calls `Host::default`.
2. `run` in [[Serve Module]] calls `serve`. `serve` calls `Error::external` in [[Error]], then `state`, then `configure` in [[Semantic Module]], then `resolved` in [[Settings Module (cw-server)]], then `set_inline_budget` in [[Semantic Module]], then `resume` in [[Embed Job Module]], and 1 more. `state` calls `Jobs::default` in [[Jobs (cw-server)]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Modules:

- [[Activity Module]] — what the backend is doing right now, for `GET /v1/status`.
- [[Api Module]] — routes and handlers.
- [[Claims Module]] — which waiting notes an agent holds.
- [[Embed Job Module]] — the background embedding job: the one long job that may run.
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
- [[Service Module]] — uses `DOCUMENT_KINDS`, `open_existing_index_in`
- [[Error]] — uses `external`, `invalid`
- [[LocalEmbedder]] — uses `LocalEmbedder`, `default`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `hmac_sha256`

## Used by

- [[Serve Module]] — from `run`
- [[Inner]] — from `Inner`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

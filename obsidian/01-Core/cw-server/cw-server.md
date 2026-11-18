---
tags: [core, cw-server]
---

# cw-server

> HTTP API of the CodeWiki backend.

## Source

- `crates/cw-server/Cargo.toml` — package manifest
- `crates/cw-server/src/lib.rs` — crate root

## How it works

Builds a library.

Modules:

- [[Activity Module]] — what the backend is doing right now, for `GET /v1/status`.
- [[Api Module]] — routes and handlers.
- [[Repos Module]] — the list of repositories this backend manages, stored as `repos.json`.
- [[Settings Module (cw-server)]] — what the settings routes do: describe the settings in effect, and change the saved ones.
- [[Workspace Module (cw-server)]] — views across every repository the backend manages.

Defined at the crate root:

- `ServerConfig` — struct
- `router` — build the router.
- `serve` — serve until `shutdown` resolves or a client calls `POST /v1/shutdown`.
- `WEBHOOK_SECRET_ENV` — environment variable that holds the webhook secret.
- [[Inner]] — struct
- [[AppState]] — struct
- `state` — function

## Depends on

- [[cw-core]] — uses `CLIENT_HEADER`, `MCP_CLIENT`, `VERSION`
- [[codewiki-app]] — uses `body`
- [[cw-llm]] — uses `header`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `hmac_sha256`
- [[Logs Module]] — uses `flush`
- [[Api Module]] — uses `routes`
- [[Settings Module (cw-server)]] — uses `resolved`

## Used by

- [[Serve Module]] — from `run`
- [[Api Module]] — from `routes`
- [[Inner]] — from `Inner`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

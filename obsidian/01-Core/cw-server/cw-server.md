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
- [[Models Module]] — the built-in embedding models, as the model routes see them: what there is, what is installed, and the one long job that may run.
- [[Repos Module]] — the list of repositories this backend manages, stored as `repos.json`.
- [[Settings Module (cw-server)]] — what the settings routes do: describe the settings in effect, and change the saved ones.
- [[Workspace Module (cw-server)]] — views across every repository the backend manages.

Defined at the crate root:

- [[Host]] — what the work on built-in models asks of the machine.
- `ServerConfig` — struct
- `router` — build the router.
- `serve` — serve until `shutdown` resolves or a client calls `POST /v1/shutdown`.
- `WEBHOOK_SECRET_ENV` — environment variable that holds the webhook secret.
- [[AppState]] — struct
- [[Inner]] — struct
- `state` — function

## Depends on

- [[cw-core]] — uses `CLIENT_HEADER`, `MCP_CLIENT`, `VERSION`
- [[Semantic Module]] — uses `behind`, `configure_in`, `set_inline_budget`
- [[Jobs]] — uses `close`, `default`, `status`
- [[Service Module]] — uses `DOCUMENT_KINDS`, `open_existing_index_in`
- [[LocalEmbedder]] — uses `LocalEmbedder`, `default`
- [[Host]] — uses `Host`, `default`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `hmac_sha256`

## Used by

- [[Serve Module]] — from `run`
- [[Inner]] — from `Inner`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

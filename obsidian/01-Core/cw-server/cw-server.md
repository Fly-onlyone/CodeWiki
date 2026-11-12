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

- [[Api Module]] — routes and handlers.
- [[Repos Module]] — the list of repositories this backend manages, stored as `repos.json`.
- [[Workspace Module (cw-server)]] — views across every repository the backend manages.

Defined at the crate root:

- [[ServerConfig]] — struct
- `router` — build the router.
- `serve` — serve until `shutdown` resolves.
- `WEBHOOK_SECRET_ENV` — environment variable that holds the webhook secret.
- [[Inner]] — struct
- [[AppState]] — struct

## Depends on

- [[codewiki-app]] — uses `body`
- [[cw-llm]] — uses `header`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `hmac_sha256`
- [[Api Module]] — uses `routes`
- [[Error]] — uses `external`
- [[AppState]] — uses `AppState`
- [[Inner]] — uses `Inner`

## Used by

- [[Serve Module]] — from `run`
- [[Api Module]] — from `routes`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

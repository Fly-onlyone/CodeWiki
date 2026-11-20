---
tags: [core, cw-client]
---

# Backend

> `Backend` struct defined in `cw_client` with 18 methods.

## Source

- `crates/cw-client/src/lib.rs` — definition of `Backend`

## How it works

Declared as `pub struct Backend`.

Public API:

- `pub fn status(&self) -> Result<Option<Value>, String>` — what the backend is doing (`GET /v1/status`): the wiki updates that are running and when a bridge last called.
- `pub fn connect() -> Result<Self, String>` — find a backend, starting a local one if needed.
- `pub fn is_remote(&self) -> bool` — whether the backend was named, by `CODEWIKI_URL` or in the app's settings, rather than found or started on this machine.
- `pub fn url(&self) -> &str` — base URL of the backend this client talks to.
- `pub fn answers(&self) -> bool` — whether the backend still answers `/v1/health`.
- `pub fn origin(&self) -> Origin` — how this client came to its backend.

## Depends on

- [[cw-client]] — uses `BACKEND_FILE`, `Origin`, `STARTUP_TIMEOUT`, `STATUS_TIMEOUT`
- [[Logs Module]] — uses `LOG_DIR_ENV`, `dir`, `path`
- [[cw-core]] — uses `CLIENT_HEADER`, `VERSION`
- [[Remote Module]] — uses `saved_in`, `token_in`
- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki-app]] — from `AppView`, `backend_to_stop`, `connect`, `connect_remote`
- [[codewiki-mcp]] — from `Bridge`, `call`, `connected`
- [[cw-client]] — from `check_remote_in`, `health_version`

## See also

- [[_index]]
- [[cw-client]]

---
tags: [core, cw-client]
---

# Backend

> `Backend` struct defined in `cw_client` with 12 methods.

## Source

- `crates/cw-client/src/lib.rs` — definition of `Backend`

## How it works

Declared as `pub struct Backend`.

Public API:

- `pub fn connect() -> Result<Self, String>` — find a backend, starting a local one if needed.
- `pub fn url(&self) -> &str` — base URL of the backend this client talks to.
- `pub fn answers(&self) -> bool` — whether the backend still answers `/v1/health`.
- `pub fn repo_id(&self, path: &str) -> Result<String, String>` — id of the repository at `path`, registering it on first use.
- `pub fn shutdown(&self) -> Result<(), String>` — ask the backend to stop once its running requests are done.
- `pub fn stop(&self) -> bool` — stop the backend and wait until it no longer answers.

## Depends on

- [[cw-client]] — uses `BACKEND_FILE`, `STARTUP_TIMEOUT`, `STOP_TIMEOUT`, `TOKEN_ENV`
- [[cw-core]] — uses `VERSION`
- [[cw-llm]] — uses `header`
- [[Logs Module]] — uses `path`
- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki-app]] — from `AppView`, `backend_to_stop`, `connect`, `forget_repo`
- [[codewiki-mcp]] — from `Bridge`, `call`, `connected`
- [[Api Module]] — from `routes`

## See also

- [[_index]]
- [[cw-client]]

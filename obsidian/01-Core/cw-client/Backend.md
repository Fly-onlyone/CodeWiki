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
- `pub fn url(&self) -> &str` — base URL of the backend this client talks to.
- `pub fn connect() -> Result<Self, String>` — find a backend, starting a local one if needed.
- `pub fn answers(&self) -> bool` — whether the backend still answers `/v1/health`.
- `pub fn origin(&self) -> Origin` — how this client came to its backend.
- `pub fn repo_id(&self, path: &str) -> Result<String, String>` — id of the repository at `path`, registering it on first use.

## Depends on

- [[cw-client]] — uses `BACKEND_FILE`, `Origin`, `STARTUP_TIMEOUT`, `STATUS_TIMEOUT`
- [[Logs Module]] — uses `LOG_DIR_ENV`, `dir`, `path`
- [[cw-core]] — uses `CLIENT_HEADER`, `VERSION`
- [[Remote Module]] — uses `saved_in`, `token_in`
- [[cw-llm]] — uses `header`
- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki-app]] — from `AppView`, `backend_to_stop`, `connect`, `connect_remote`
- [[codewiki-mcp]] — from `Bridge`, `call`, `connected`
- [[cw-client]] — from `check_remote_in`, `health_version`
- [[cw-server]] — from `call_as`, `hook`
- [[Api Module]] — from `log_request`, `routes`
- [[cw-embed]] — from `post_json`
- [[Provider Module]] — from `post_json`

## See also

- [[_index]]
- [[cw-client]]

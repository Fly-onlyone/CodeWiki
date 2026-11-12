---
tags: [core, cw-client]
---

# Backend

> `Backend` struct defined in `cw_client` with 8 methods.

## Source

- `crates/cw-client/src/lib.rs` — definition of `Backend`

## How it works

Declared as `pub struct Backend`.

Public API:

- `pub fn connect() -> Result<Self, String>` — find a backend, starting a local one if needed.
- `pub fn url(&self) -> &str` — base URL of the backend this client talks to.
- `pub fn repo_id(&self, path: &str) -> Result<String, String>` — id of the repository at `path`, registering it on first use.
- `pub fn get(&self, path: &str) -> Result<Value, String>`
- `pub fn post(&self, path: &str, body: &Value) -> Result<Value, String>`
- `pub fn put(&self, path: &str, body: &Value) -> Result<Value, String>`

## Depends on

- [[cw-client]] — uses `BACKEND_FILE`, `STARTUP_TIMEOUT`, `TOKEN_ENV`, `URL_ENV`
- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki-app]] — from `AppView`, `connect`, `open_settings`, `run`
- [[codewiki-mcp]] — from `Bridge`, `call`, `call_global`
- [[Api Module]] — from `routes`

## See also

- [[_index]]
- [[cw-client]]

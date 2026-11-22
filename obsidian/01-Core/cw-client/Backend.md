---
tags: [core, cw-client]
---

# Backend

> `Backend` struct defined in `cw_client` with 18 methods.

## Source

- `crates/cw-client/src/lib.rs` — definition of `Backend`

## How it works

1. `AppView::connect` and `AppView::restart_backend` in [[codewiki-app]], and 1 more, call `connect`. `connect` calls `data_dir` in [[Paths Module]], then `named`, then `read_backend_file`, then `health_version` (both in [[cw-client]]), then `new`, then `is_older` in [[cw-client]], and 4 more. `named` calls `new`, then `saved_in`, then `token_in` (both in [[Remote Module]]), then `health_version` in [[cw-client]].
2. `check_remote_in` in [[cw-client]] calls `new`. `new` calls `agent` in [[cw-client]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct Backend`.

Members:

- `fn named( data_dir: &Path, env: impl Fn(&str) -> Option<String>, ) -> Option<Result<Self, String>>` — the backend that was named: by `CODEWIKI_URL`, or else by the app's settings. `None` when neither names one, and the backend on this machine is the one to use. A named backend that cannot be used is an error, never a reason to fall back to this machine: the user chose where their wikis are, and a local backend started in its place would show other ones, or none.
- `pub fn connect() -> Result<Self, String>` — find a backend, starting a local one if needed.
- `pub fn url(&self) -> &str` — base URL of the backend this client talks to.
- `pub fn origin(&self) -> Origin` — how this client came to its backend.
- `pub fn is_remote(&self) -> bool` — whether the backend was named, by `CODEWIKI_URL` or in the app's settings, rather than found or started on this machine. A named backend is not this client's to stop.
- `pub fn shutdown(&self) -> Result<(), String>` — ask the backend to stop once its running requests are done.
- `pub fn answers(&self) -> bool` — whether the backend still answers `/v1/health`.
- `pub fn stop(&self) -> bool` — stop the backend and wait until it no longer answers. Returns false when it could not be asked or is still running after `STOP_TIMEOUT`.
- `fn new(url: String, token: String, origin: Origin) -> Self` — private method, returns `Self`; called by `Backend::connect`, `Backend::named`, `check_remote_in`.
- `pub fn with_client(mut self, client: &'static str) -> Self` — the same backend, with every request saying what this client is. The MCP bridge passes `MCP_CLIENT`, so the backend can tell the app when an agent last called. The app itself sends nothing.
- `fn authorized<B>(&self, request: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B>` — `request` with the token, and with what this client is when it says.
- `pub fn status(&self) -> Result<Option<Value>, String>` — what the backend is doing (`GET /v1/status`): the wiki updates that are running and when a bridge last called. `None` from a backend too old to have the route.
- `pub fn get(&self, path: &str) -> Result<Value, String>` — public method, returns `Result<Value, String>`; called by `Backend::repo_id`, `Backend::status`.
- `pub fn put(&self, path: &str, body: &Value) -> Result<Value, String>` — public method, returns `Result<Value, String>`.
- `pub fn post(&self, path: &str, body: &Value) -> Result<Value, String>` — public method, returns `Result<Value, String>`; called by `Backend::repo_id`, `Backend::shutdown`.
- `pub fn delete(&self, path: &str) -> Result<Value, String>` — public method, returns `Result<Value, String>`.
- `pub fn repo_id(&self, path: &str) -> Result<String, String>` — id of the repository at `path`, registering it on first use.

And 1 more private item with no comment.

## Depends on

- [[cw-client]] — uses `BACKEND_FILE`, `Origin`, `STARTUP_TIMEOUT`, `STATUS_TIMEOUT`
- [[Logs Module]] — uses `LOG_DIR_ENV`, `dir`, `path`
- [[cw-core]] — uses `CLIENT_HEADER`, `VERSION`
- [[Remote Module]] — uses `saved_in`, `token_in`
- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki-app]] — from `AppView`, `backend_to_stop`, `connect`, `connect_remote`
- [[codewiki-mcp]] — from `Bridge`, `call`, `connected`, `get_visible`
- [[cw-client]] — from `check_remote_in`, `health_version`

## See also

- [[_index]]
- [[cw-client]]

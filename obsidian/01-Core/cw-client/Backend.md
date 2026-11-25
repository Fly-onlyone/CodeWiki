---
tags: [core, cw-client]
---

# Backend

> `Backend` struct defined in `cw_client` with 26 methods.

## Source

- `crates/cw-client/src/lib.rs` — definition of `Backend`

## How it works

1. `AppView::restart_backend` and `reach` in [[codewiki-app]], and 1 more, call `connect`. `connect` calls `data_dir` in [[Paths Module]], then `named`, then `running_in`, then `is_older` in [[cw-client]], then `stop`, then `start`. `named` calls `new`, then `saved_in`, then `token_in` (both in [[Remote Module]]), then `health_version` in [[cw-client]]. `running_in` calls `read_backend_file`, then `health_version` (both in [[cw-client]]), then `new`.
2. `reach` in [[codewiki-app]] calls `find`. `find` calls `find_in`, then `data_dir` in [[Paths Module]]. `find_in` calls `named`, then `running_in`.
3. `check_remote_in` in [[cw-client]] calls `new`. `new` calls `agent` in [[cw-client]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct Backend`.

Members:

- `fn named( data_dir: &Path, env: impl Fn(&str) -> Option<String>, ) -> Option<Result<Self, String>>` — the backend that was named: by `CODEWIKI_URL`, or else by the app's settings. `None` when neither names one, and the backend on this machine is the one to use. A named backend that cannot be used is an error, never a reason to fall back to this machine: the user chose where their wikis are, and a local backend started in its place would show other ones, or none.
- `pub fn find() -> Result<Option<Self>, String>` — find a backend that runs, and never start one: the one named by `CODEWIKI_URL` or in the app's settings, or else the one on this machine that `backend.json` names, when it answers `/v1/health`. `Ok(None)` when nothing is named and no backend runs here.
- `fn find_in( data_dir: &Path, env: impl Fn(&str) -> Option<String>, ) -> Result<Option<Self>, String>` — private method, returns `Result<Option<Self>, String>`; called by `Backend::find`.
- `fn running_in(data_dir: &Path) -> Option<(Self, String)>` — the backend that `backend.json` in `data_dir` names, with the version it reports, when it answers.
- `pub fn connect() -> Result<Self, String>` — find a backend, starting a local one if needed.
- `pub fn url(&self) -> &str` — base URL of the backend this client talks to.
- `pub fn origin(&self) -> Origin` — how this client came to its backend.
- `pub fn is_remote(&self) -> bool` — whether the backend was named, by `CODEWIKI_URL` or in the app's settings, rather than found or started on this machine. A named backend is not this client's to stop.
- `pub fn shutdown(&self) -> Result<(), String>` — ask the backend to stop once its running requests are done.
- `pub fn answers(&self) -> bool` — whether the backend still answers `/v1/health`.
- `pub fn stop(&self) -> bool` — stop the backend and wait until it no longer answers. Returns false when it could not be asked or is still running after `STOP_TIMEOUT`.
- `fn new(url: String, token: String, origin: Origin) -> Self` — private method, returns `Self`; called by `Backend::named`, `Backend::running_in`, `check_remote_in`.
- `pub fn with_client(mut self, client: &'static str) -> Self` — the same backend, with every request saying what this client is. The MCP bridge passes `MCP_CLIENT`, so the backend can tell the app when an agent last called. The app itself sends nothing.
- `pub fn status(&self) -> Result<Option<Value>, String>` — what the backend is doing (`GET /v1/status`): the wiki updates that are running and when a bridge last called. `None` from a backend too old to have the route.
- `pub fn get(&self, path: &str) -> Result<Value, String>` — public method, returns `Result<Value, String>`; called by `Backend::repo_id`, `Backend::status`.
- `pub fn put(&self, path: &str, body: &Value) -> Result<Value, String>` — public method, returns `Result<Value, String>`.
- `pub fn post(&self, path: &str, body: &Value) -> Result<Value, String>` — public method, returns `Result<Value, String>`; called by `Backend::decide_proposals`, `Backend::repo_id`, `Backend::shutdown`.
- `pub fn delete(&self, path: &str) -> Result<Value, String>` — public method, returns `Result<Value, String>`.
- `pub fn repo_id(&self, path: &str) -> Result<String, String>` — id of the repository at `path`, registering it on first use.
- `decide_proposals` — approve (`approve` true) or reject several proposals of `repo` in one call. One result per id, in the order given: a proposal that cannot be decided does not stop the others. An older backend has no such route; the error says so, and the proposals can still be decided one by one.

And 6 more items.

## Depends on

- [[cw-client]] — uses `DecideResult`, `Origin`, `SHUTDOWN_CONNECT_TIMEOUT`, `STARTS`
- [[cw-core]] — uses `CLIENT_HEADER`, `VERSION`
- [[Remote Module]] — uses `saved_in`, `token_in`
- [[Logs Module]] — uses `path`
- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki-app]] — from `AppView`, `Reached`, `backend_to_stop`, `connect_remote`
- [[codewiki-mcp]] — from `Bridge`, `call`, `connected`, `get_visible`
- [[cw-client]] — from `check_remote_in`, `health_version`
- [[Review View Module]] — from `decide`

## See also

- [[_index]]
- [[cw-client]]

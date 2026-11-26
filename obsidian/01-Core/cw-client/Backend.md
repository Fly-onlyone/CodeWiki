---
tags: [core, cw-client]
---

# Backend

> `Backend` struct defined in `cw_client` with 26 methods.

## What it does

`Backend` is the handle a client holds to talk to the CodeWiki backend over HTTP. It knows the backend's address, its bearer token and how the client came to it (`Origin`: named by `CODEWIKI_URL`, saved in the app's settings, or local). The app and the MCP bridge in [[codewiki-mcp]] both use it, so neither keeps an index of its own.

## Source

- `crates/cw-client/src/lib.rs` — definition of `Backend`

## How it works

`connect` picks a backend in a fixed order.

1. `named` first. `CODEWIKI_URL` wins, and needs `CODEWIKI_TOKEN`. Otherwise a backend saved in settings is used when marked in use, after its token is read ([[Remote Module]]) and `/v1/health` answers.
2. Else `running_in` reads `backend.json` in the data directory and checks health. If that backend reports an older version than this program (`is_older`), `stop` replaces it. A newer or unreadable version is kept.
3. Else `start` spawns `codewiki serve` and waits up to 30 seconds for `backend.json` to name a backend that answers.

`find` is the same lookup but never starts anything and returns `Ok(None)` when nothing runs.

Starts are guarded by a process-wide `STARTS` lock. Only one start runs at a time. A second caller waits for the one started last, as long as that process runs and has never been named in `backend.json`. If it ends without answering, one new start replaces it, even when two callers wait.

Requests go through `get`, `put`, `post` and `delete`. Each adds the `Authorization` bearer header, and a client that called `with_client` also sends the client header. An error answer becomes the backend's own `error` text, or `backend returned <status>`. `repo_id` registers a path, and when that fails on a remote backend it matches known repositories by id or folder name.

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

## Gotchas

- A named backend that fails is an error. `connect` never falls back to a local backend, because that would show other wikis or none.
- `stop` refuses nothing for remote origins, so callers check `is_remote` first: a named backend is not the client's to stop.
- `shutdown` limits connecting to 100 ms. The app asks as it quits and GPUI waits only 200 ms; a refused Windows connection otherwise takes about 2 seconds.
- `stop` waits up to 30 seconds, because the backend finishes running requests first.
- `hold_starts(true)` makes `start` fail while an update replaces the programs.
- The spawned backend gets no console window on Windows. A console made it wait 147 s to serve in a clean-Windows test.
- Requests time out after 600 seconds, since generation on a large repository can take minutes. `status` uses 3 seconds.
- `decide_proposals` turns a 404 or 405 with no error body into a message to update the backend.

## See also

- [[_index]]
- [[cw-client]]

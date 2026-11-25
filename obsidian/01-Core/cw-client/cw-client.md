---
tags: [core, cw-client]
---

# cw-client

> Client for the CodeWiki backend: discovery, autostart and HTTP calls.

## What it does

Clients hold no index of their own. They find a running backend, starting a local one when none is running, and send every request to it.

## Source

- `crates/cw-client/Cargo.toml` — package manifest
- `crates/cw-client/src/lib.rs` — crate root

## How it works

Builds a library.

1. `AppView::open_file` and `AppView::open_in_obsidian` in [[codewiki-app]], and 19 more, call `encode`.
2. `Backend::new` and `Backend::status` in [[Backend]] call `agent`.
3. `AppView::connect` and `AppView::run_settings_action` in [[codewiki-app]], and 1 more, call `env_remote`. `env_remote` calls `process_env`.
4. `AppView::backend_to_stop` and `AppView::install_update` in [[codewiki-app]] call `uses_remote`. `uses_remote` calls `env_remote`, then `saved_remote`.
5. `AppView::connect` and `AppView::open_settings` in [[codewiki-app]], and 2 more, call `saved_remote`. `saved_remote` calls `saved_in` in [[Remote Module]], then `data_dir` in [[Paths Module]].

8 more entry points: `check_remote`, `save_remote`, `use_local`, `forget_remote`, `read_backend_file`, `health_version`, and 2 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Modules:

- [[Remote Module]] — A backend on another machine, or in Docker, that the user chose in the app instead of naming it with `CODEWIKI_URL`.

Defined at the crate root:

- `const URL_ENV: &str` — explicit backend address, for a backend running in Docker or on another machine. Requires `CODEWIKI_TOKEN`.
- `pub enum Origin` — how a client came to the backend it uses.
- [[Backend]] — struct
- `const STATUS_TIMEOUT: Duration` — how long `GET /v1/status` may take. It is asked every few seconds and answers from memory, so a backend that takes longer is not answering.
- `const SHUTDOWN_CONNECT_TIMEOUT: Duration` — how long asking the backend to stop may take to connect. The backend a client stops is on this machine, which accepts at once while it runs. A refused connection takes about two seconds on Windows, and the app asks as it quits, where GPUI waits only 200 ms.
- `fn agent(timeout: Duration) -> ureq::Agent` — private function, returns `ureq::Agent`; called by `Backend::new`, `Backend::status`, `health_version`.
- `fn process_env(name: &str) -> Option<String>` — private function, returns `Option<String>`; called by `Backend::connect`, `env_remote`.
- `pub fn env_remote() -> Option<String>` — the address `CODEWIKI_URL` names, when it is set. It wins over a saved backend.
- `pub fn uses_remote() -> bool` — whether clients use a backend in Docker or on another machine, named by `CODEWIKI_URL` or saved in the app's settings, rather than one on this machine. Such a backend is not this user's to stop.
- `pub fn saved_remote() -> Option<Remote>` — the backend saved in the app's settings, without its token.
- `pub fn check_remote(url: &str, token: Option<&str>) -> Result<String, String>` — check that a CodeWiki backend answers at `url` and accepts `token`, and return its version. `token: None` checks with the saved token. Nothing is saved.
- `pub fn save_remote(url: &str, token: Option<&str>) -> Result<(), String>` — save a backend's address and token and make every client on this machine use it: the app and an agent's MCP bridge alike. `token: None` keeps the token saved for the same address. Check it with `check_remote` first.
- `pub fn use_local() -> Result<(), String>` — go back to the backend on this machine. The saved address and token are kept for the next time.
- `pub fn forget_remote() -> Result<(), String>` — forget the saved backend and its token.
- `pub fn encode(value: &str) -> String` — Percent-encode a query-string value.
- `fn read_backend_file(data_dir: &Path) -> Option<(String, String)>` — private function, returns `Option<(String, String)>`; called by `Backend::connect`.
- `fn health_version(url: &str) -> Option<String>` — the version a backend reports, when it answers `/v1/health`. A backend that answers without a version reports an empty string.
- `fn is_older(theirs: &str, ours: &str) -> bool` — whether a backend reporting `theirs` is older than this program. One that cannot be compared counts as not older, so it is kept.
- `const STOP_TIMEOUT: Duration` — how long an old backend may take to stop after it is asked to: it first finishes the requests it is running.
- `fn codewiki_binary() -> PathBuf` — the `codewiki` binary: beside this executable when installed together, otherwise whatever `PATH` provides.

And 4 more items.

## Depends on

- [[Remote Module]] — uses `Remote`, `forget_in`, `normalized`, `save_in`
- [[Backend]] — uses `new`, `status`
- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki-app]] — from `backend_to_stop`, `connect`, `connect_remote`, `install_update`
- [[codewiki-mcp]] — from `get_architecture`, `get_blast_radius`, `get_context`, `get_dependency_path`
- [[Backend]] — from `Backend`, `answers`, `connect`, `named`
- [[Markdown Module]] — from `encode`
- [[Settings Pages Module]] — from `Model`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

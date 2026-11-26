---
tags: [core, cw-client]
---

# cw-client

> Client for the CodeWiki backend: discovery, autostart and HTTP calls.

## What it does

The library every CodeWiki client uses to reach the backend: the desktop app and the MCP bridge. Clients keep no index. This crate finds a running backend, starts a local one when none runs, and sends HTTP requests with the bearer token. It also holds the settings for using a backend on another machine, and the helper `encode` for query strings. [[Backend]] is the main type; [[Remote Module]] stores a saved remote backend.

## Source

- `crates/cw-client/Cargo.toml` — package manifest
- `crates/cw-client/src/lib.rs` — crate root

## How it works

The crate answers one question: which backend does this client use? The order is: `CODEWIKI_URL` (with `CODEWIKI_TOKEN`), then the backend saved in settings, then the one on this machine named by `backend.json`, and last a new `codewiki serve` it spawns itself. `env_remote`, `saved_remote` and `uses_remote` report the first two, so the app knows a backend is not its own to stop.

`check_remote` tests an address and token before saving: `/v1/health` must answer, then a call to `/v1/repos` proves the token. It saves nothing. `save_remote`, `use_local` and `forget_remote` pass through to [[Remote Module]].

`health_version` reads the version from `/v1/health`. `is_older` compares versions with semver, so `connect` can replace an older local backend left from before an update.

Starting is serialized by a process-wide lock (`Starts`). It tracks the child started last, whether a start is under way, and whether starts are held. `hold_starts` lets the app block new backends during an update. `spawn_backend` runs `codewiki serve` beside this executable, or from `PATH`, with the data and log folders set in its environment.

`DecideResult` and `decide_proposals` support approving or rejecting several wiki proposals in one request.

## Items

Modules:

- [[Remote Module]] — A backend on another machine, or in Docker, that the user chose in the app instead of naming it with `CODEWIKI_URL`.

Defined at the crate root:

- `pub enum Origin` — how a client came to the backend it uses.
- [[Backend]] — struct
- `fn agent(timeout: Duration) -> ureq::Agent` — private function, returns `ureq::Agent`; called by `Backend::new`, `Backend::status`, `health_version`.
- `fn process_env(name: &str) -> Option<String>` — private function, returns `Option<String>`; called by `Backend::connect`, `Backend::find`, `env_remote`.
- `pub fn env_remote() -> Option<String>` — the address `CODEWIKI_URL` names, when it is set. It wins over a saved backend.
- `pub fn uses_remote() -> bool` — whether clients use a backend in Docker or on another machine, named by `CODEWIKI_URL` or saved in the app's settings, rather than one on this machine. Such a backend is not this user's to stop.
- `pub fn saved_remote() -> Option<Remote>` — the backend saved in the app's settings, without its token.
- `pub fn check_remote(url: &str, token: Option<&str>) -> Result<String, String>` — check that a CodeWiki backend answers at `url` and accepts `token`, and return its version. `token: None` checks with the saved token. Nothing is saved.
- `pub fn save_remote(url: &str, token: Option<&str>) -> Result<(), String>` — save a backend's address and token and make every client on this machine use it: the app and an agent's MCP bridge alike. `token: None` keeps the token saved for the same address. Check it with `check_remote` first.
- `pub fn use_local() -> Result<(), String>` — go back to the backend on this machine. The saved address and token are kept for the next time.
- `pub fn forget_remote() -> Result<(), String>` — forget the saved backend and its token.
- `pub fn encode(value: &str) -> String` — Percent-encode a query-string value.
- `fn read_backend_file(data_dir: &Path) -> Option<(String, String)>` — private function, returns `Option<(String, String)>`; called by `Backend::running_in`.
- `fn health_version(url: &str) -> Option<String>` — the version a backend reports, when it answers `/v1/health`. A backend that answers without a version reports an empty string.
- `fn is_older(theirs: &str, ours: &str) -> bool` — whether a backend reporting `theirs` is older than this program. One that cannot be compared counts as not older, so it is kept.
- `fn lock(starts: &Mutex<Starts>) -> MutexGuard<'_, Starts>` — private function, returns `MutexGuard<'_, Starts>`; called by `Backend::start_in`, `spawn_claimed`, `starts`.
- `pub fn hold_starts(held: bool)` — keep this process from starting a backend, or let it again. The app holds starts while an update replaces the programs: a backend started then would run the program being replaced.
- `spawn_claimed` — make the start that `Starts::claim` took on, and keep its child.
- `fn spawn_backend(data_dir: &Path) -> Result<Child, String>` — start `codewiki serve` in the background.
- `pub struct DecideResult` — what the backend did with one proposal of a batch.

And 15 more items.

## Depends on

- [[Remote Module]] — uses `Remote`, `forget_in`, `normalized`, `save_in`
- [[Logs Module]] — uses `LOG_DIR_ENV`, `dir`
- [[Paths Module]] — uses `DATA_DIR_ENV`, `data_dir`
- [[Backend]] — uses `new`, `status`

## Used by

- [[codewiki-app]] — from `backend_to_stop`, `connect`, `connect_remote`, `install_update`
- [[Backend]] — from `Backend`, `answers`, `connect`, `decide_proposals`
- [[codewiki-mcp]] — from `get_architecture`, `get_blast_radius`, `get_context`, `get_dependency_path`
- [[Review Module]] — from `decided_notices`, `written`
- [[Markdown Module]] — from `encode`
- [[Review View Module]] — from `decide`
- [[Settings Pages Module]] — from `Model`

## Gotchas

- The spawned backend's output is discarded. Its own `backend.log` has the reports. A pipe nobody reads would fill and stall it.
- A newer backend is never stopped by an older client.
- An unparsable version counts as not older, so that backend is kept.
- The backend child is started with no console window on Windows.
- `check_remote` needs a token: passed in, or saved for the same address.
- Error answers are not turned into transport errors. The agent is built with `http_status_as_error(false)` so the JSON message reaches the caller.

## See also

- [[_index]]
- [[Crate Dependency Flow]]

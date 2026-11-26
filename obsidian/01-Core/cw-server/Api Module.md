---
tags: [core, cw-server, module]
---

# Api Module

> Routes and handlers.

## What it does

This is the HTTP face of the backend. `routes` builds one axum router with every `/v1` endpoint: repositories, notes, search, graph, risk, health, proposals, settings and embedding jobs. The app, the MCP bridge and the git webhook all call it. Each handler opens the repository, does the work and returns JSON.

## Source

- `crates/cw-server/src/api.rs` — module `cw_server::api`

## How it works

`routes` makes two groups. The protected group sits under `/v1` behind `require_token`, which wants an `Authorization: Bearer` token and compares it in constant time. A request that also carries the MCP client header counts as a bridge call, and only after the token passed. Outside that layer sit `/v1/health` and `/v1/hooks/git`. The webhook proves itself by signature instead.

The whole router is wrapped by `log_request` and `trace_request`. The log shows method, path, status and time, never the query, headers or body. The trace names a request by method and route, never by path, and skips `/v1/health` and `/v1/status`, which clients poll.

Handlers do blocking work (SQLite, files, parsing) through `blocking`, which uses `spawn_blocking`. Errors become JSON `{error}` with 404, 400 or 500.

Writes take the shared write lock. `run_update` registers a running update first, so a waiting update still shows as running, then takes the lock, updates, and calls `notes_changed`.

`git_hook` checks the secret (GitHub HMAC or GitLab token), matches the pushed repository name to a registered folder name, and answers 202. The update runs detached afterwards, because git hosts wait only seconds.

The explanation routes let agents list and write waiting notes. Listing and lending happen under one claims lock, so two agents never get the same note. See [[Claims]] and [[Activity]].

## Items

- `blocking` — run blocking work (SQLite, file I/O, parsing) off the async runtime.
- `const SLOW_REQUEST: Duration` — A request that takes longer than this is logged at Info.
- `fn request_level(path: &str, status: StatusCode, elapsed: Duration) -> log::Level` — the level a finished request is logged at. Failures of the backend stand out; refused and slow requests are kept; the rest, and the health checks clients poll with, show only at Debug.
- `const UNTRACED: [&str; 2]` — routes that clients poll every few seconds. Traced, they would bury the requests worth looking at.
- `fn trace_name(method: &str, route: Option<&str>) -> Option<String>` — the name a request is traced under: its method and its route, never its path, which holds ids and names. `None` for a request that is not traced.
- `pub fn routes(state: AppState) -> Router` — public function, returns `Router`; called by `router`, `serve`.
- `fn lock_claims(state: &AppState) -> std::sync::MutexGuard<'_, crate::claims::Claims>` — the claims on waiting notes. Take it inside the write lock, never the other way round, and hold it briefly.
- `async fn backend_status(State(state): State<AppState>) -> Json<Value>` — what the backend is doing: the wiki updates that are running, whoever started them, and when an agent's MCP bridge last called. The app shows it in its tray icon. Unlike `/v1/health` it needs the token.
- `fn pending_proposals(data_dir: &std::path::Path, found: &mut dyn FnMut(String, pending::Found))` — count the pending proposals of each registered repository, by repository id, and tell each to `found`: `None` for one whose folder is gone or whose vault cannot be read.
- `fn embed_closed(state: &AppState) -> Option<Response>` — the answer of every embedding job route on a backend that listens beyond this machine. The job runs on the backend's machine and calls the model server its settings name; like the settings, that is decided there.
- `struct Repo` — resolved repository: its root folder and its index.
- `struct ListedRepo` — A registered repository as `GET /v1/repos` lists it. `present` is worked out on each request, on this machine, where the paths are; it is never written to `repos.json`.
- `fn run_update(state: &AppState, id: &str) -> Result<cw_wiki::Report>` — Re-index a repository and bring its wiki up to date, holding the write lock. Shared by the update route and the git webhook.
- `const WEBHOOK_BODY_LIMIT: usize` — largest webhook body accepted; GitHub caps push payloads at 25 MB.
- `fn webhook_signed(secret: &str, headers: &HeaderMap, body: &[u8]) -> bool` — true when the request proves knowledge of the webhook secret: GitHub's `X-Hub-Signature-256` (HMAC-SHA256 of the raw body, hex) or GitLab's `X-Gitlab-Token` (the secret itself).
- `fn pushed_names(payload: &Value) -> Vec<String>` — folder names a push payload may refer to: GitHub's `repository.name` and `repository.full_name`, GitLab's `project.name` and `project.path_with_namespace`. Namespaced names contribute their last segment.
- `struct NoteReadQuery` — what a read of a note asks for. A write names the note alone.
- `struct ExplanationsQuery` — which waiting notes a coding agent asks for.
- `const MAX_AGENT_LEN: usize` — the longest agent id accepted.
- `struct WriteExplanationQuery` — which note an agent writes, and who it is.

And 85 more items.

## Depends on

- [[Query Module]] — uses `Answer`, `Context`, `NoteContent`, `NoteInfo`
- [[Explanations Module]] — uses `Brief`, `Outcome`, `Queue`, `Scope`
- [[Gitintel Module]] — uses `BlastRadius`, `ChangeRisk`, `FileRisk`, `GitIntel`
- [[Proposal Module]] — uses `Proposal`, `get`, `list`, `patch_text`
- [[Index]] — uses `Index`, `edges`, `files`, `find_symbols`
- [[Coverage Module]] — uses `CoverageReport`, `import`, `import_text`, `load`
- [[Health Module]] — uses `DeadSymbol`, `DocReport`, `FileHealth`, `dead_code`
- [[Claims]] — uses `Claims`, `claim`, `expire`, `others`

## Used by

- [[cw-server]] — from `router`, `serve`

## Gotchas

- Lock order matters. Take the claims lock inside the write lock, never the other way round, and hold it briefly.
- `backend_status` uses `try_lock` on the claims, so it never waits. If a lister holds them, expiry is skipped.
- `/v1/status` never fails and never waits long for the proposal count. A slow count is left out of that answer.
- `write_explanation` answers 200 for every outcome, even a list of faults, because a client keeps only one line of an error.
- The embedding routes answer 403 when settings are not writable here, meaning the backend listens beyond this machine.
- The webhook body is limited to 25 MB.
- Without a webhook secret, the hook answers 404 as if it did not exist.
- `present` in the repo list is worked out per request and never saved.

## See also

- [[_index]]
- [[cw-server]]

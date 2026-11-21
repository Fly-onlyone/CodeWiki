---
tags: [core, cw-server, module]
---

# Api Module

> Routes and handlers.

## Source

- `crates/cw-server/src/api.rs` — module `cw_server::api`

## How it works

1. `router` and `serve` in [[cw-server]] call `routes`.

## Items

- `blocking` — run blocking work (SQLite, file I/O, parsing) off the async runtime.
- `const SLOW_REQUEST: Duration` — A request that takes longer than this is logged at Info.
- `fn request_level(path: &str, status: StatusCode, elapsed: Duration) -> log::Level` — the level a finished request is logged at. Failures of the backend stand out; refused and slow requests are kept; the rest, and the health checks clients poll with, show only at Debug.
- `const UNTRACED: [&str; 2]` — routes that clients poll every few seconds. Traced, they would bury the requests worth looking at.
- `fn trace_name(method: &str, route: Option<&str>) -> Option<String>` — the name a request is traced under: its method and its route, never its path, which holds ids and names. `None` for a request that is not traced.
- `pub fn routes(state: AppState) -> Router` — public function, returns `Router`; called by `router`, `serve`.
- `async fn backend_status(State(state): State<AppState>) -> Json<Value>` — what the backend is doing: the wiki updates that are running, whoever started them, and when an agent's MCP bridge last called. The app shows it in its tray icon. Unlike `/v1/health` it needs the token.
- `async fn embed_models(State(state): State<AppState>) -> Api<Value>` — the models the chosen embedder's server offers. Always 200: a server that does not answer is reported in the body.
- `fn embed_closed(state: &AppState) -> Option<Response>` — the answer of every embedding job route on a backend that listens beyond this machine. The job runs on the backend's machine and calls the model server its settings name; like the settings, that is decided there.
- `async fn cancel_embed_job(State(state): State<AppState>) -> Response` — stop the running job. What it embedded stays, and the next job goes on from there.
- `struct Repo` — resolved repository: its root folder and its index.
- `struct ListedRepo` — A registered repository as `GET /v1/repos` lists it. `present` is worked out on each request, on this machine, where the paths are; it is never written to `repos.json`.
- `fn run_update(state: &AppState, id: &str) -> Result<cw_wiki::Report>` — Re-index a repository and bring its wiki up to date, holding the write lock. Shared by the update route and the git webhook.
- `const WEBHOOK_BODY_LIMIT: usize` — largest webhook body accepted; GitHub caps push payloads at 25 MB.
- `fn webhook_signed(secret: &str, headers: &HeaderMap, body: &[u8]) -> bool` — true when the request proves knowledge of the webhook secret: GitHub's `X-Hub-Signature-256` (HMAC-SHA256 of the raw body, hex) or GitLab's `X-Gitlab-Token` (the secret itself).
- `fn pushed_names(payload: &Value) -> Vec<String>` — folder names a push payload may refer to: GitHub's `repository.name` and `repository.full_name`, GitLab's `project.name` and `project.path_with_namespace`. Namespaced names contribute their last segment.
- `async fn git_hook(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response` — push webhook from a git host: update the wiki of the registered repository whose folder is named like the pushed one. The update runs after the response is sent; a host waits only a few seconds for an answer.
- `struct NoteReadQuery` — what a read of a note asks for. A write names the note alone.
- `struct ExplanationsQuery` — which waiting notes a coding agent asks for.
- `coverage` — coverage imported earlier; without one, a report found in a conventional place in the repository is read but not stored.

And 79 more items.

## Depends on

- [[Query Module]] — uses `Answer`, `Context`, `NoteContent`, `NoteInfo`
- [[Gitintel Module]] — uses `BlastRadius`, `ChangeRisk`, `FileRisk`, `GitIntel`
- [[Explanations Module]] — uses `Brief`, `Outcome`, `Queue`, `Scope`
- [[Index]] — uses `Index`, `edges`, `files`, `find_symbols`
- [[Proposal Module]] — uses `Proposal`, `get`, `list`, `patch_text`
- [[Coverage Module]] — uses `CoverageReport`, `import`, `import_text`, `load`
- [[Health Module]] — uses `DeadSymbol`, `DocReport`, `FileHealth`, `dead_code`
- [[Repos Module]] — uses `RepoEntry`, `add`, `get`, `load`

## Used by

- [[cw-server]] — from `router`, `serve`

## See also

- [[_index]]
- [[cw-server]]

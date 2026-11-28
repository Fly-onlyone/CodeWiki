---
tags: [integration]
---

# axum

> External crate `axum`, imported by 4 source files.

## What it does

axum is the web framework behind the CodeWiki backend. It maps each HTTP route to a handler function, passes shared state to the handlers, and wraps routes in middleware. The backend serves data only, as JSON. The app, the MCP bridge and a git host are its clients.

## Used for

Imported by 4 source files. Main users:

- [[cw-server]] — imports `Router`, `body::Body`, `http::Request`, `http::StatusCode`
- [[Api Module]] — imports `Json`, `Router`, `body::Bytes`, `extract::DefaultBodyLimit` and 13 more

## Configuration

- `crates/cw-server/Cargo.toml` — declares the dependency: version `0.8.9`, locked `0.8.9`

## Wire-up

`routes` in the API module builds the `Router`. Handlers are plain async functions. They take extractors such as `State`, `Path`, `Query`, `Json` and `Bytes`. Errors become JSON `{ "error": ... }` with a status chosen by `ApiError`.

The router has two parts.

- A protected router holds every route except two: `/repos`, `/repos/{id}/...`, `/workspace`, `/status`, `/settings`, `/embed/...`, `/shutdown` and the rest. The `require_token` middleware wraps it, and it is nested under `/v1`.
- The outer router holds `/v1/health` and `/v1/hooks/git`, which sit outside the token layer. The hook route has its own body limit of 25 MB.

Two more layers wrap the outer router: `log_request` and, outside it, `trace_request`, so what the log reports joins the trace. `with_state` hands the shared [[AppState]] to all of it.

`router` returns the `Router` without a socket. The tests call it through `tower`'s `oneshot`, so a test sends one request straight to the app. `serve` binds a TCP listener, publishes the bound address, configures semantic search, resumes the background embedding job, and runs `axum::serve` with a graceful shutdown. Slow work such as SQLite or file access goes through `blocking`, which uses `spawn_blocking`, so the async runtime stays free.

## Auth mode

Every protected route needs the header `Authorization: Bearer <token>`. `require_token` compares it with the configured token using `constant_time_eq`. A mismatch gets HTTP 401. The token comes from the server config, never from this code.

The git webhook cannot send that token. It is signed instead. `webhook_signed` accepts GitHub's `X-Hub-Signature-256` (an HMAC of the raw body) or GitLab's `X-Gitlab-Token`, both checked against the webhook secret. Without a secret in the config, the route answers 404.

## Gotchas

- A route added outside the protected router has no token check. Add new routes to the protected one unless a signature replaces the token.
- The bridge call is counted only after the token passes. Anyone can send the client header.
- The request log and trace record method and route only, never the query, headers or body. They carry search text, notes and the token. The trace uses the matched route (with `{id}`), not the real path, so ids stay out of it. `/v1/health` and `/v1/status` are not traced, because clients poll them.
- Log levels: server errors are warnings, health checks stay at debug, refused or slow requests (over one second) are info.
- The webhook answers 202 and runs the update afterwards. A git host waits only a few seconds.
- Without a webhook secret, the hook route answers 404, not 401.
- Updates, whether from the update route or the webhook, share one write lock. A second update waits, yet is already counted as running.
- Route parameters use the `{id}` form of this axum version.

## See also

- [[_index]]

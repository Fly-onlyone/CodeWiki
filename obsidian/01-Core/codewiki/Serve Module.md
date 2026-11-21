---
tags: [core, codewiki, module]
---

# Serve Module

> `codewiki serve`: run the backend HTTP server.

## Source

- `crates/codewiki/src/serve.rs` — module `codewiki::serve`

## How it works

1. `run` in [[codewiki]] calls `run`. `run` calls `data_dir` in [[Paths Module]], then `Error::invalid` in [[Error]], then `random_token`, then `Host::default` in [[Host]], then `drop_retired`, then `retire_builtin` (both in [[Settings Module (cw-core)]]), and 8 more. `random_token` calls `Error::external` in [[Error]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const TOKEN_ENV: &str` — environment variable carrying a fixed access token (used in Docker).
- `pub const BACKEND_FILE: &str` — file in the data directory that tells local clients how to connect.
- `fn random_token() -> Result<String>` — private function, returns `Result<String>`; called by `run`.
- `fn publish(data_dir: &Path, addr: SocketAddr, token: &str) -> Result<()>` — tell local clients (the app, the MCP bridge) where the backend listens.
- `fn names_process(text: &str, pid: u32) -> bool` — whether the connection file `text` was written by the process `pid`.
- `fn withdraw(data_dir: &Path)` — delete the connection file, if it is still this backend's. A backend asked to stop first finishes its running requests; a client that finds it not listening may by then have started the next backend, whose file must stay.
- `pub fn run(host: IpAddr, port: u16) -> Result<()>` — public function, returns `Result<()>`; called by `run`.

## Depends on

- [[Settings Module (cw-core)]] — uses `RETIRED_VARIABLES`, `drop_retired`, `key`, `process_env`
- [[cw-server]] — uses `ServerConfig`, `WEBHOOK_SECRET_ENV`, `serve`
- [[Error]] — uses `external`, `invalid`, `io`
- [[Semantic Module]] — uses `configure`, `embedder`
- [[Error Module]] — uses `Result`
- [[Paths Module]] — uses `data_dir`
- [[Resolved]] — uses `load`
- [[Host]] — uses `default`

## Used by

- [[codewiki]] — from `run`

## See also

- [[_index]]
- [[codewiki]]

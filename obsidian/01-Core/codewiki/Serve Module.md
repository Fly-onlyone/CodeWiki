---
tags: [core, codewiki, module]
---

# Serve Module

> `codewiki serve`: run the backend HTTP server.

## What it does

This is the `codewiki serve` command. It starts the backend: an HTTP server that the desktop app, the MCP bridge and other clients talk to. It picks the access token, tells local clients where to connect, runs the server until Ctrl+C, and removes its connection file on exit.

## Source

- `crates/codewiki/src/serve.rs` — module `codewiki::serve`

## How it works

`run` does these steps:

1. It takes the token from `CODEWIKI_TOKEN`. If that is unset, it makes a random 64-character hex token, but only when the host is a loopback address. On any other address it refuses to start.
2. It builds a `ServerConfig`. Settings may be changed only when the host is loopback.
3. It cleans up removed features. It drops the old AI writer's settings and moves the removed built-in embedder to `local`, deleting its models. It logs a warning for any retired environment variable that is still set.
4. It loads the settings and configures the embedder, so the first log line names the real one.
5. It builds a tokio runtime and calls `cw_server::serve`. When the server is bound, `publish` writes `backend.json` into the data directory with the URL, the token and the process id.
6. After Ctrl+C the server finishes its running requests, `withdraw` removes the file, and the runtime shuts down within five seconds.

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
- [[cw-server]] — uses `ServerConfig`, `WEBHOOK_SECRET_ENV`, `default`, `serve`
- [[Error]] — uses `external`, `invalid`, `io`
- [[Semantic Module]] — uses `configure`, `embedder`
- [[Error Module]] — uses `Result`
- [[Paths Module]] — uses `data_dir`
- [[Resolved]] — uses `load`

## Used by

- [[codewiki]] — from `run`

## Gotchas

- `withdraw` deletes `backend.json` only if its pid is this process. A client may already have started the next backend while this one was finishing, and that file must stay.
- A host that is not loopback needs `CODEWIKI_TOKEN`. Without it the command fails at once, so no one is tempted to turn auth off.
- A bind to an unspecified address (all interfaces) publishes `127.0.0.1` as the host, because that is where local clients can connect.
- The runtime is shut down with a five second limit. A job's embedder batch may be in a blocking task that only ends when the embedding server answers, and Quit waits five seconds at most.
- The connection file holds the token in plain text. Treat the data directory as private.

## See also

- [[_index]]
- [[codewiki]]

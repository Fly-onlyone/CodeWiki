---
tags: [core, codewiki, module]
---

# Serve Module

> `codewiki serve`: run the backend HTTP server.

## Source

- `crates/codewiki/src/serve.rs` — module `codewiki::serve`

## How it works

Main items:

- `BACKEND_FILE` — file in the data directory that tells local clients how to connect.
- `TOKEN_ENV` — environment variable carrying a fixed access token (used in Docker).
- `run` — function
- `names_process` — whether the connection file `text` was written by the process `pid`.
- `publish` — tell local clients (the app, the MCP bridge) where the backend listens.
- `random_token` — function
- `withdraw` — delete the connection file, if it is still this backend's.

## Depends on

- [[Error]] — uses `external`, `invalid`, `io`
- [[cw-server]] — uses `WEBHOOK_SECRET_ENV`, `serve`
- [[Error Module]] — uses `Result`
- [[Paths Module]] — uses `data_dir`
- [[ServerConfig]] — uses `ServerConfig`

## Used by

- [[codewiki]] — from `run`

## See also

- [[_index]]
- [[codewiki]]

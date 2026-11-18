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
- `writer_name` — the LLM provider the settings in effect choose, by name; never its key.

## Depends on

- [[cw-server]] — uses `ServerConfig`, `WEBHOOK_SECRET_ENV`, `serve`
- [[Error]] — uses `external`, `invalid`, `io`
- [[Semantic Module]] — uses `configure`, `embedder`
- [[Resolved]] — uses `Resolved`, `load`
- [[Error Module]] — uses `Result`
- [[Paths Module]] — uses `data_dir`
- [[Settings Module (cw-core)]] — uses `process_env`
- [[Provider Module]] — uses `from_settings`

## Used by

- [[codewiki]] — from `run`

## See also

- [[_index]]
- [[codewiki]]

---
tags: [core, cw-core, module]
---

# Logs Module

> Log files of the programs, under `<data dir>/logs`.

## Source

- `crates/cw-core/src/logs.rs` — module `cw_core::logs`

## How it works

Main items:

- `level_in_env` — the level `CODEWIKI_LOG` names, when it names one.
- `dir` — the folder of the log files: `CODEWIKI_LOG_DIR` when set, otherwise `<data dir>/logs`.
- `level_named` — the level `name` names, as `CODEWIKI_LOG` and the settings spell it.
- `path` — the log file of program `name`: `<log dir>/<name>.log`.
- `LOG_DIR_ENV` — environment variable naming the folder of the log files.
- `LIMIT` — largest a log file grows before it is moved aside.
- `LEVEL_ENV` — environment variable setting the level: `error`, `warn`, `info`, `debug`, `trace` or `off`.
- `timestamp` — `time` in UTC as `2026-11-13T20:03:13.123Z`, the form every log line starts with.

## Depends on

- [[cw-core]] — uses `VERSION`
- [[Paths Module]] — uses `data_dir`
- [[Resolved]] — uses `load`

## Used by

- [[codewiki-app]] — from `connect`, `main`, `open_logs`, `run_settings_action`
- [[Worker Module]] — from `refuse`, `serve`
- [[codewiki]] — from `main`
- [[codewiki-mcp]] — from `main`
- [[cw-embed]] — from `serve_once`
- [[cw-embed-engine]] — from `worker`
- [[cw-llm]] — from `serve_once`
- [[cw-server]] — from `stand_in`

## See also

- [[_index]]
- [[cw-core]]

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
- `LEVEL_ENV` — environment variable setting the level: `error`, `warn`, `info`, `debug`, `trace` or `off`.
- `LOG_DIR_ENV` — environment variable naming the folder of the log files.
- `LIMIT` — largest a log file grows before it is moved aside.
- `init` — send the `log` records of program `name` at `default` and above (or at the level in `CODEWIKI_LOG`) to `<log dir>/<name>.log` and to standard error, and log every panic.

## Depends on

- [[cw-core]] — uses `VERSION`
- [[Diagrams Module]] — uses `store`
- [[Paths Module]] — uses `data_dir`
- [[ListItemButton]] — uses `level`
- [[Resolved]] — uses `load`

## Used by

- [[codewiki-app]] — from `connect`, `main`, `open_logs`, `run_settings_action`
- [[codewiki]] — from `main`
- [[codewiki-mcp]] — from `main`
- [[cw-embed]] — from `serve_once`
- [[cw-llm]] — from `serve_once`
- [[cw-server]] — from `stand_in`
- [[Download Module]] — from `fetch`
- [[Wire Tests Module]] — from `serve`

## See also

- [[_index]]
- [[cw-core]]

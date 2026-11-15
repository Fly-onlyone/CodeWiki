---
tags: [core, codewiki-app, module]
---

# Logfile Module

> The app's log: `%LOCALAPPDATA%\CodeWiki\logs\app.log`.

## Source

- `crates/codewiki-app/src/logfile.rs` — module `codewiki_app::logfile`

## How it works

Main items:

- `init` — send `log` records and panics to the app's log file.
- `FileLogger` — struct
- `Sink` — the open file and how long it is, so that it can be moved aside once full.
- `level_from` — the level asked for in `CODEWIKI_LOG`, Info when unset or unknown.

## Depends on

- [[Logs Module]] — uses `LIMIT`, `open`, `path`, `timestamp`
- [[cw-core]] — uses `VERSION`
- [[ListItemButton]] — uses `level`

## Used by

- [[codewiki-app]] — from `main`
- [[cw-embed]] — from `serve_once`
- [[cw-llm]] — from `serve_once`
- [[Download Module]] — from `fetch`
- [[Wire Tests Module]] — from `serve`

## See also

- [[_index]]
- [[codewiki-app]]

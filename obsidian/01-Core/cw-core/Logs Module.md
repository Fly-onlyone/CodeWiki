---
tags: [core, cw-core, module]
---

# Logs Module

> Log files of the programs, under `<data dir>/logs`.

## Source

- `crates/cw-core/src/logs.rs` — module `cw_core::logs`

## How it works

Main items:

- `path` — the log file of program `name`: `<data dir>/logs/<name>.log`.
- `open` — open the log at `path` for appending, creating its folder, after moving it aside when it is over `LIMIT`.
- `rotate` — move `path` to `<name>.old.log` beside it when it is over `LIMIT`.
- `LIMIT` — largest a log file grows before it is moved aside.
- `timestamp` — `time` in UTC as `2026-11-13T20:03:13.123Z`, the form every log line starts with.
- `civil_date` — the calendar date `days` days after 1970-01-01, in the proleptic Gregorian calendar (Howard Hinnant's `civil_from_days`).

## Depends on

- [[Paths Module]] — uses `data_dir`

## Used by

- [[Logfile Module]] — from `init`, `log`, `open`
- [[codewiki-app]] — from `connect`
- [[cw-client]] — from `backend_log`
- [[Backend]] — from `connect`

## See also

- [[_index]]
- [[cw-core]]

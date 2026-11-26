---
tags: [core, cw-core, module]
---

# Logs Module

> Log files of the programs, under `<data dir>/logs`.

## What it does

Gives every CodeWiki program a log file. The desktop app and the backend it starts have no console, so without files their reports would be lost. Each program appends to `<log dir>/<name>.log`. Past 1 MB (`LIMIT`) the file is moved to `<name>.old.log`, replacing the older one, so at most two files per program exist. A program run from a terminal also sees the lines on standard error.

## Source

- `crates/cw-core/src/logs.rs` — module `cw_core::logs`

## How it works

`init` (or `init_with`) builds a `Logger`, sets its level from `CODEWIKI_LOG` or the default given, and installs it as the process logger. It also wraps the panic hook so every panic is logged first, then passed on. A start line with the version, arguments and pid is logged. `init_with` lets [[cw-telemetry]] wrap the file logger inside one that also sends records to Sentry.

`init_console` installs a logger that prints only to standard error, for a command run in a terminal. The `codewiki` program uses it for all commands but `serve`.

`dir` is `CODEWIKI_LOG_DIR` (made absolute, so a program started elsewhere finds it) or `<data dir>/logs` from [[Paths Module]]. `path(name)` adds the file name.

The level lives in an atomic cell, so `set_level` can change it while the program runs and also sets the `log` crate's maximum level. The app's settings use `level_named` and `level_name` for the spelling: off, error, warn, info, debug, trace.

Every line is `timestamp level target: message`, with a UTC timestamp in milliseconds written by `timestamp`.

## Items

- `pub const LIMIT: u64` — largest a log file grows before it is moved aside.
- `pub const LOG_DIR_ENV: &str` — environment variable naming the folder of the log files. A relative path is taken from the working folder.
- `pub const LEVEL_ENV: &str` — environment variable setting the level: `error`, `warn`, `info`, `debug`, `trace` or `off`.
- `pub fn dir() -> PathBuf` — the folder of the log files: `CODEWIKI_LOG_DIR` when set, otherwise `<data dir>/logs`.
- `fn dir_from(value: Option<OsString>) -> PathBuf` — private function, returns `PathBuf`; called by `dir`.
- `pub fn path(name: &str) -> PathBuf` — the log file of program `name`: `<log dir>/<name>.log`.
- `pub fn open(path: &Path) -> io::Result<File>` — open the log at `path` for appending, creating its folder, after moving it aside when it is over `LIMIT`.
- `pub fn rotate(path: &Path) -> bool` — move `path` to `<name>.old.log` beside it when it is over `LIMIT`. Returns whether it was moved. The file must not be open in this process, or Windows refuses the move.
- `struct Level` — the level of a logger, which can change while the program runs.
- `struct Logger` — A `log` logger that appends to a file and echoes to standard error.
- `pub fn level_from(value: Option<&str>, default: LevelFilter) -> LevelFilter` — the level named by `value` (the value of `CODEWIKI_LOG`), or `default` when it is unset or unknown.
- `pub fn level_named(name: &str) -> Option<LevelFilter>` — the level `name` names, as `CODEWIKI_LOG` and the settings spell it.
- `pub fn level_name(level: LevelFilter) -> &'static str` — the name of `level`, as `CODEWIKI_LOG` and the settings spell it.
- `pub fn level_in_env() -> Option<LevelFilter>` — the level `CODEWIKI_LOG` names, when it names one. It wins over a level the program would choose itself.
- `pub fn current_level() -> LevelFilter` — the level of the installed logger: records at it and above are kept.
- `pub fn set_level(level: LevelFilter)` — change the level of the logger `init` installed, from the next record on. Nothing happens in a program that installed none.
- `pub fn init(name: &str, default: LevelFilter) -> Option<PathBuf>` — send the `log` records of program `name` at `default` and above (or at the level in `CODEWIKI_LOG`) to `<log dir>/<name>.log` and to standard error, and log every panic. Call first in `main`. Returns the file's path, or `None` when the folder is not writable (the records then go to standard error only) or a logger was installed already.
- `init_with` — `init`, with the file logger passed through `wrap` before it is installed: `cw-telemetry` puts it inside the logger that also hands records to Sentry. The level stays the file logger's, so `set_level` still changes it.
- `pub fn init_console(default: LevelFilter)` — send `log` records at `default` and above (or at the level in `CODEWIKI_LOG`) to standard error only, for a command run in a terminal.
- `pub fn timestamp(time: SystemTime) -> String` — `time` in UTC as `2026-11-13T20:03:13.123Z`, the form every log line starts with.

And 8 more items.

## Depends on

- [[cw-core]] — uses `VERSION`
- [[Paths Module]] — uses `data_dir`
- [[Resolved]] — uses `load`

## Used by

- [[codewiki-app]] — from `connect`, `main`, `open_logs`, `run_settings_action`
- [[cw-telemetry]] — from `flush`, `init`
- [[codewiki]] — from `main`
- [[cw-client]] — from `spawn_backend`
- [[cw-embed]] — from `answer`
- [[cw-server]] — from `stand_in`
- [[Backend]] — from `start_in`

## Gotchas

- `rotate` needs the file closed in this process: Windows will not move an open file. The logger closes and reopens its file before rotating. If another process holds the file, the move fails and the next try waits for another megabyte of growth, instead of retrying on every line.
- Only one logger can be installed. A second `init` returns `None` and changes nothing.
- `init` returns `None` also when the folder is not writable. Records then go to standard error only.
- `CODEWIKI_LOG` wins over a default level a program chooses, and an unknown value is ignored.
- A release test script finds the start line by the words `started with`. Keep that text.

## See also

- [[_index]]
- [[cw-core]]

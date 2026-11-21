---
tags: [core, cw-core, module]
---

# Logs Module

> Log files of the programs, under `<data dir>/logs`.

## What it does

The desktop app has no console, and the backend it starts has none either, so what they report would otherwise be lost. Each program appends to its own file. A file that grows past `LIMIT` is moved aside to `<name>.old.log`, replacing the one before, so a program never keeps more than two.

`init` sends a program's `log` records to its file and to standard error, so that a program run from a terminal or an IDE shows them there as well. `CODEWIKI_LOG_DIR` moves the folder (a development run puts it in the repository); `CODEWIKI_LOG` sets the level.

## Source

- `crates/cw-core/src/logs.rs` — module `cw_core::logs`

## How it works

1. `AppView::connect` and `AppView::open_logs` in [[codewiki-app]], and 2 more, call `dir`. `dir` calls `dir_from`. `dir_from` calls `data_dir` in [[Paths Module]].
2. `AppView::connect` in [[codewiki-app]] and `Backend::connect` in [[Backend]] call `path`. `path` calls `dir`.
3. `main` and `AppView::run_settings_action` in [[codewiki-app]] call `level_named`.
4. `AppView::settings_frame` in [[codewiki-app]] calls `level_name`.
5. `AppView::settings_frame` in [[codewiki-app]] calls `level_in_env`. `level_in_env` calls `level_named`.

4 more entry points: `current_level`, `set_level`, `init_with`, `init_console`.

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
- [[cw-embed]] — from `answer`
- [[cw-server]] — from `stand_in`
- [[Backend]] — from `connect`

## See also

- [[_index]]
- [[cw-core]]

---
tags: [core, cw-telemetry]
---

# cw-telemetry

> Error reports of CodeWiki's programs to Sentry: opt-in, scrubbed of paths and request data.

## What it does

Error reports to Sentry from the programs that run for a long time: the app, the backend, the MCP bridge and the embedding worker.

A program reports only when one of these holds:

- `SENTRY_DSN` is set in its environment, as on a developer's machine;
- the program was built with a DSN (`CODEWIKI_SENTRY_DSN` at compile time, which the release workflow sets) and the user turned on "Send error reports" in Settings → Diagnostics, which writes `CONSENT_FILE` in `%APPDATA%\CodeWiki`.

Each program reads that file itself, so the MCP bridge and a backend an agent started follow the switch too. Turning it on takes effect at each program's next start. Turning it off holds at once, because every report is checked against the file again first.

`init` replaces `cw_core::logs::init`: records still go to the log file, and an Error record also becomes a report, while Info and Warn records go along with the next report as its trail. A panic is reported with its backtrace. Before anything leaves the machine it is scrubbed (`scrub`): no host name, no absolute path, and no query string, headers or body of a request.

## Source

- `crates/cw-telemetry/Cargo.toml` — package manifest
- `crates/cw-telemetry/src/lib.rs` — crate root

## How it works

Builds a library.

1. `main` and `sentry_test` in [[codewiki]], and 3 more, call `init`. `init` calls `start`, then `init_with` in [[Logs Module]]. `start` calls `choose`, then `env_dsn`, then `built_in_dsn`, then `consent`, then `environment`, then `traces_rate`, and 3 more.
2. `sentry_test` in [[codewiki]] and `trace_request` in [[Api Module]] call `sending`. `sending` calls `allowed`. `allowed` calls `switch_on`.
3. `AppView::settings_frame` in [[codewiki-app]] calls `reporting`. `reporting` calls `env_dsn`, then `built_in_dsn`.
4. `AppView::settings_frame` in [[codewiki-app]] calls `consent`. `consent` calls `consent_path`, then `read_consent`.
5. `AppView::run_action` in [[codewiki-app]] calls `set_consent`. `set_consent` calls `consent_path`, then `write_consent`.

3 more entry points: `flush`, `send_test_report`, `task_hub`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Modules:

- [[Scrub Module]] — what is taken out of a report before it leaves the machine.

Defined at the crate root:

- `pub const DSN_ENV: &str` — environment variable naming the project to report to. It needs no switch: whoever sets it chose to report.
- `pub const ENVIRONMENT_ENV: &str` — environment variable naming the environment of the reports. Without it, `development` for a debug build and `production` for a release build.
- `pub const TRACES_RATE_ENV: &str` — environment variable with the share of backend requests traced, from 0 to 1. `DEFAULT_TRACES_RATE` without it.
- `pub const DEFAULT_TRACES_RATE: f32` — the share of backend requests traced when `TRACES_RATE_ENV` is unset.
- `pub const CONSENT_FILE: &str` — the file in `%APPDATA%\CodeWiki` that holds the user's choice: `on`, or absent.
- [[Reporting]] — whether and how this program can report.
- `pub fn reporting() -> Reporting` — how this program can report, for the Settings page.
- `fn built_in_dsn() -> Option<&'static str>` — private function, returns `Option<&'static str>`; called by `reporting`, `start`.
- `pub fn consent_path() -> Option<PathBuf>` — where the user's choice is kept: `%APPDATA%\CodeWiki\error_reports.txt`.
- `pub fn consent() -> bool` — whether the user turned on "Send error reports".
- `fn read_consent(path: &Path) -> bool` — private function, returns `bool`; called by `consent`.
- `pub fn set_consent(on: bool) -> io::Result<()>` — keep the user's choice. Off removes the file.
- `fn allowed() -> bool` — whether a report may leave now: always under `SENTRY_DSN`, and under the switch while it is still on.
- `fn switch_on() -> bool` — the switch, read again at most every `RECHECK`: a backend traces requests, and need not read a file for each.
- `pub fn sending() -> bool` — whether this process sends reports now: `init` started them and the user has not turned them off since. The backend traces a request only then.
- `pub struct Guard` — keeps the reports going; hold it until `main` returns, whose end sends what is still queued.
- `pub fn init(name: &'static str, default: LevelFilter) -> Guard` — start the logs of program `name` as `cw_core::logs::init` does, and its error reports when they are allowed (see the crate's documentation). Call first in `main`.
- `pub fn flush()` — send what is queued, waiting up to two seconds. For a program that is about to end by a path that skips the end of `main`.
- `pub fn send_test_report(name: &str)` — A report that a check can look for: an Error record, which becomes an event when reports are on.
- `pub fn task_hub() -> Arc<sentry::Hub>` — the hub a task of its own reports through, so that what one request adds to its scope stays out of the others.

And 10 more items.

## Depends on

- [[Logs Module]] — uses `flush`, `init_with`
- [[Scrub Module]] — uses `breadcrumb`, `event`
- [[cw-core]] — uses `VERSION`
- [[Reporting]] — uses `Reporting`

## Used by

- [[codewiki-app]] — from `main`, `run_action`, `settings_frame`
- [[codewiki]] — from `main`, `sentry_test`
- [[codewiki-mcp]] — from `main`
- [[cw-embed-engine]] — from `worker`
- [[Settings Pages Module]] — from `error_reports`
- [[Api Module]] — from `trace_request`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

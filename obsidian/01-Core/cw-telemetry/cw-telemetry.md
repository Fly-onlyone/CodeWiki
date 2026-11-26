---
tags: [core, cw-telemetry]
---

# cw-telemetry

> Error reports of CodeWiki's programs to Sentry: opt-in, scrubbed of paths and request data.

## What it does

Sends error reports to Sentry from the long-running programs: the app, the backend and the MCP bridge. It is opt-in. A program reports only if `SENTRY_DSN` is set in its environment, or if the build carries a DSN and the user turned on Send error reports in Settings. Everything is scrubbed first, see [[Scrub Module]].

## Source

- `crates/cw-telemetry/Cargo.toml` — package manifest
- `crates/cw-telemetry/src/lib.rs` — crate root

## How it works

`init` replaces the plain log setup. `start` calls `choose`: the environment DSN always wins, and the built-in one is used only when the consent file says `on`. A DSN that does not parse means no reports, not a crash. The mode is stored once in `MODE`. Sentry is set up with no personal data, a stack trace on every event, and the trace rate from the environment (default 0.1, bad values ignored).

The log layer maps records. An Error record becomes a report. Info and Warn become the trail (breadcrumbs). Debug and Trace are dropped. Panics are left to Sentry's own hook. Records from the sending crates are ignored, so reporting cannot report on itself. Error records of a recovered graphics device loss are only trail.

Every event passes `before_send`, which asks `allowed` again and then calls `scrub::event`. In switch mode `allowed` reads the consent file, but at most every five seconds. The consent file holds `on` or is absent. `set_consent` writes or removes it. `sending` tells whether this process reports now, and the backend traces a request only then. `task_hub` gives each request its own Sentry hub, so scopes do not mix.

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
- `pub fn consent_path() -> Option<PathBuf>` — where the user's choice is kept: `%APPDATA%\CodeWiki\error_reports.txt`, or under `CodeWiki-Dev` in a development build (`cw_core::paths::APP_FOLDER`).
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

And 12 more items.

## Depends on

- [[Logs Module]] — uses `flush`, `init_with`
- [[Scrub Module]] — uses `breadcrumb`, `event`
- [[cw-core]] — uses `VERSION`
- [[Paths Module]] — uses `APP_FOLDER`
- [[Reporting]] — uses `Reporting`

## Used by

- [[codewiki-app]] — from `main`, `run_action_with`, `settings_frame`
- [[codewiki]] — from `main`, `sentry_test`
- [[codewiki-mcp]] — from `main`
- [[Settings Pages Module]] — from `error_reports`
- [[Api Module]] — from `trace_request`

## Gotchas

- Turning reports on takes effect at the next start of each program. Turning them off holds within about five seconds, with no restart.
- `SENTRY_DSN` needs no switch. Whoever sets it chose to report, and the Settings switch does not stop it.
- The release workflow may set the built-in DSN empty. An empty value counts as no DSN.
- Hold the `Guard` until `main` ends. A program that exits another way should call `flush` first, which waits up to two seconds.
- Development builds keep the consent file under a separate folder name.

## See also

- [[_index]]
- [[Crate Dependency Flow]]

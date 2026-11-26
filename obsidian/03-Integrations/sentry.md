---
tags: [integration]
---

# sentry

> External crate `sentry`, imported by 3 source files.

## What it does

Sentry is an error tracking service. The `sentry` crate sends it error reports, panics and request traces from the long-running programs: the app, the backend and the MCP bridge. All of it goes through the `cw-telemetry` crate, so the rest of the code does not set Sentry up by itself. Reports are off unless the environment or the user's choice turns them on.

## Used for

Imported by 3 source files. Main users:

- [[Api Module]] — imports `protocol::SpanStatus`
- [[Scrub Module]] — imports `protocol::Breadcrumb`, `protocol::DebugImage`, `protocol::Event`
- [[cw-telemetry]] — imports `integrations::log::LogFilter`, `integrations::log::RecordMapping`, `integrations::log::SentryLogger`, `integrations::log::breadcrumb_from_record` and 1 more

## Configuration

- `crates/cw-telemetry/Cargo.toml` — declares the dependency: version `0.49.6`, locked `0.49.6`, features `backtrace`, `contexts`, `debug-images`, `log`, `panic`, `rustls`, `ureq`, without its default features

## Wire-up

Each program calls `init` first in `main`. It replaces the plain log setup. Log records still go to the log file. With reporting on, `init` puts Sentry's logger around the file logger.

- An Error record becomes a report. Info and Warn records go with the next report as its trail.
- A panic is reported by Sentry's own hook, with a backtrace. The log record of the panic is left out.
- Records from `sentry`, `ureq` and `rustls` are ignored, so the sender never reports on itself.
- A graphics device loss that the UI recovers from is trail only.

`init` returns a `Guard`. Hold it until `main` returns, because its drop sends what is queued.

The backend adds request traces. `trace_request` in the API module starts a transaction named by method and route, and sets its status from the HTTP status. `trace_name` skips the polled routes `/v1/health` and `/v1/status`. The trace rate is 0.1 unless `SENTRY_TRACES_SAMPLE_RATE` holds a number from 0 to 1.

The `scrub` module runs on every event and breadcrumb before they leave. It drops the host name and user, and the query, headers, cookies and body of a request. It replaces every absolute path in a message with a placeholder.

## Auth mode

The only credential is the DSN, a project address. It comes from the `SENTRY_DSN` variable, or from the build, where the release workflow sets `CODEWIKI_SENTRY_DSN` at compile time. A DSN from the environment always reports. A built-in DSN reports only while the user's switch is on, kept as the file `error_reports.txt` in the app folder. This note holds no DSN value.

## Gotchas

- Turning the switch on takes effect at the next start. Turning it off holds at once, since each event is checked before it is sent.
- `send_default_pii` is false, and the scrub rules run in `before_send`. A new field on an event that holds user text needs a scrub rule.
- A DSN that does not parse means no reports, with no error.
- The environment name is `development` for a debug build and `production` for a release, unless `SENTRY_ENVIRONMENT` is set.
- Request traces are recorded only while reports are sending.

## See also

- [[_index]]

---
tags: [integration]
---

# sentry

> External crate `sentry`, imported by 3 source files.

## What it does

Sentry (sentry.io) client for Rust.

## Used for

Imported by 3 source files. Main users:

- [[Api Module]] — imports `protocol::SpanStatus`
- [[Scrub Module]] — imports `protocol::Breadcrumb`, `protocol::DebugImage`, `protocol::Event`
- [[cw-telemetry]] — imports `integrations::log::LogFilter`, `integrations::log::SentryLogger`

## Configuration

- `crates/cw-telemetry/Cargo.toml` — declares the dependency: version `0.49.6`, locked `0.49.6`, features `backtrace`, `contexts`, `debug-images`, `log`, `panic`, `rustls`, `ureq`, without its default features

## See also

- [[_index]]

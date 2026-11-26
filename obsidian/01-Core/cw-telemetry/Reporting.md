---
tags: [core, cw-telemetry]
---

# Reporting

> Whether and how this program can report.

## What it does

Says whether this program can send error reports to Sentry, and what controls it. The Settings page reads it to decide what to show under Diagnostics: a switch, a note that the environment decides, or nothing.

## Source

- `crates/cw-telemetry/src/lib.rs` — definition of `Reporting`

## How it works

`reporting` returns one of three values.

- `Environment`: the `SENTRY_DSN` variable is set. Reports go out, whatever the user's switch says. Whoever set the variable chose to report.
- `Switch`: the build carries a DSN (set at compile time by the release workflow) and there is no `SENTRY_DSN`. The user's switch decides.
- `Unavailable`: neither exists. This build never reports.

The switch is the file `error_reports.txt` in the app folder under `%APPDATA%`. The text `on` means yes. No file means no. `choose` applies the rule at start: the environment DSN always wins, and the built-in DSN is used only when the file says `on`.

`init` stores the chosen mode in `MODE`. Later, `allowed` checks it before every report leaves. `Environment` always allows. `Switch` reads the file again, but at most every five seconds.

## Items

Declared as `pub enum Reporting`.

## Used by

- [[cw-telemetry]] — from `MODE`, `choose`, `reporting`
- [[Settings Pages Module]] — from `Model`

## Gotchas

- Turning the switch on takes effect at the next start of each program, because the client is only built in `init`.
- Turning it off holds at once, since each report is checked again before it is sent. The check may be up to five seconds behind.
- An empty or blank `SENTRY_DSN` counts as unset. A DSN that does not parse means no reports, not a crash.
- The mode is only set when reporting starts. Before `init`, or in `Unavailable`, `allowed` is false.

## See also

- [[_index]]
- [[cw-telemetry]]

---
tags: [core, codewiki]
---

# codewiki

> CodeWiki command line and backend server.

## Source

- `crates/codewiki/Cargo.toml` — package manifest
- `crates/codewiki/src/main.rs` — crate root

## How it works

Builds a binary.

1. The program starts at `main`. `main` calls `session_start_hook`, then `embed_worker`, then `sentry_test`, then `init` in [[cw-telemetry]], then `init_console` in [[Logs Module]], then `run`. `session_start_hook` calls `vault_root`, then `session_start` (both in [[Briefing Module]]). `embed_worker` calls `worker` in [[cw-embed-engine]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Modules:

- [[Model Module (codewiki)]] — `codewiki model`: the built-in embedding models from the command line.
- [[Serve Module]] — `codewiki serve`: run the backend HTTP server.

Defined at the crate root:

- `fn session_start_hook(repo: &std::path::Path)` — print the session-start briefing for the repository at `repo`. Every failure is swallowed: a hook must never break the agent's session.
- `fn print_json(encoded: serde_json::Result<String>) -> Result<()>` — print a result encoded with `serde_json::to_string_pretty`.
- `fn run(cli: Cli) -> Result<()>` — private function, returns `Result<()>`; called by `main`.
- `fn sentry_test(panic: bool) -> ExitCode` — `codewiki sentry-test`: one report through the same path an error or a panic of the backend takes.
- `fn embed_worker(model: &str, models_dir: &std::path::Path) -> ExitCode` — private function, returns `ExitCode`; called by `main`.
- `fn main() -> ExitCode` — private function, returns `ExitCode`.

And 4 more private items with no comment.

## Depends on

- [[cw-illustrate]] — uses `MAX_CALLOUTS`, `MAX_CALLOUT_TEXT`, `MAX_PICTURES`, `MAX_SOURCE`
- [[Lint Module]] — uses `CONCEPT_MAX_WORDS`, `INTEGRATION_MAX_WORDS`, `MAX_WORDS`, `MIN_WORDS`
- [[Ci Module]] — uses `check`, `fails`, `to_junit`, `to_sarif`
- [[Gitintel Module]] — uses `change_risk`, `file_risk`, `hotspots`, `load`
- [[cw-telemetry]] — uses `DSN_ENV`, `init`, `send_test_report`, `sending`
- [[Coverage Module]] — uses `import`, `load`, `report`, `save`
- [[Service Module]] — uses `is_stale`, `open_index`, `reindex`, `update`
- [[Error]] — uses `external`, `invalid`, `io`, `not_found`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

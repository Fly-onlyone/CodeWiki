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

1. The program starts at `main`. `main` calls `session_start_hook`, then `sentry_test`, then `agents`, then `init` in [[cw-telemetry]], then `init_console` in [[Logs Module]], then `run`. `session_start_hook` calls `vault_root` in [[Briefing Module]], then `data_dir` in [[Paths Module]], then `load`, then `blocking` (both in [[Agents Off Module]]), then `load` in [[Auto Explain Module]], then `hook_input`, and 3 more. `sentry_test` calls `init`, then `sending` (both in [[cw-telemetry]]), then `data_dir` in [[Paths Module]], then `send_test_report` in [[cw-telemetry]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Modules:

- [[Serve Module]] — `codewiki serve`: run the backend HTTP server.

Defined at the crate root:

- `fn agents(action: AgentsAction, json: bool) -> ExitCode` — run `codewiki agents`. A list that cannot be read is reported, never overwritten, and every action then fails.
- `enum HookAgent` — the agent host that runs a hook.
- `fn hook_input() -> Option<serde_json::Value>` — the JSON an agent host pipes to a hook, or `None` when there is none: stdin is a terminal, empty, not JSON, or does not arrive within a second. The read runs on a helper thread so a host that keeps stdin open cannot hold the session start.
- `fn session_start_hook(repo: &std::path::Path, agent: HookAgent)` — print the session-start briefing for the repository at `repo`. Every failure is swallowed: a hook must never break the agent's session.
- `fn print_json(encoded: serde_json::Result<String>) -> Result<()>` — print a result encoded with `serde_json::to_string_pretty`.
- `fn run(cli: Cli) -> Result<()>` — private function, returns `Result<()>`; called by `main`.
- `fn sentry_test(panic: bool) -> ExitCode` — `codewiki sentry-test`: one report through the same path an error or a panic of the backend takes.
- `fn main() -> ExitCode` — private function, returns `ExitCode`.

And 5 more private items with no comment.

## Depends on

- [[cw-illustrate]] — uses `MAX_CALLOUTS`, `MAX_CALLOUT_TEXT`, `MAX_PICTURES`, `MAX_SOURCE`
- [[Agents Off Module]] — uses `add`, `blocking`, `covering`, `file`
- [[Lint Module]] — uses `CONCEPT_MAX_WORDS`, `INTEGRATION_MAX_WORDS`, `MAX_WORDS`, `MIN_WORDS`
- [[Ci Module]] — uses `check`, `fails`, `to_junit`, `to_sarif`
- [[Gitintel Module]] — uses `change_risk`, `file_risk`, `hotspots`, `load`
- [[Error]] — uses `Error`, `external`, `invalid`, `io`
- [[cw-telemetry]] — uses `DSN_ENV`, `init`, `send_test_report`, `sending`
- [[Coverage Module]] — uses `import`, `load`, `report`, `save`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

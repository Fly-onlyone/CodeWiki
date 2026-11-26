---
tags: [core, codewiki]
---

# codewiki

> CodeWiki command line and backend server.

## What it does

`codewiki` is the command line program of CodeWiki, and it also runs the backend server (`codewiki serve`). From the shell a user can index a repository, generate or update the wiki, search, look up symbols, and ask for reports such as hotspots, dead code, health, risk, impacted tests, coverage and security. It handles proposals (`approve`, `reject`) and the list of folders kept from coding agents (`agents`). Almost every command takes `--repo` and `--json` before or after it.

## Source

- `crates/codewiki/Cargo.toml` — package manifest
- `crates/codewiki/src/main.rs` — crate root

## How it works

`main` parses the command line with clap. Four kinds of command are handled before a repository is resolved.

`hook session-start` is run by an agent host at the start of a session. It prints a short briefing for the repository. Any error or panic is swallowed, and the program exits with success, so a hook never breaks a session. `sentry-test` sends one test error report. `agents` edits the list of folders closed to agents. `serve` hands over to the [[Serve Module]], which runs the HTTP backend.

All other commands go through `run`. It resolves the repository root, then calls the wiki service or the index for that command and prints the result as text or, with `--json`, as pretty JSON. `print_report` shows what an update did: notes written, kept, proposed, removed, and how many explanations may be behind or are still missing.

Only `serve` sends error reports. Other commands log warnings and errors only.

The session-start hook finds the project folder from the `CLAUDE_PROJECT_DIR` variable when `--repo` is the default. It reads the agent's JSON from stdin only when auto explaining is on. `explain_for` then decides whether to ask the agent to explain waiting notes. In a linked worktree it only gives facts.

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

## Gotchas

- The hook must stay silent and succeed. Wrong arguments to `hook` also exit with success. `--help` and `--version` still print.
- A folder closed to coding agents, or one that holds a closed folder, gets no briefing. So does any run where the list cannot be read.
- The hook reads stdin on a helper thread and waits one second, up to 64 KB. A host that keeps stdin open cannot hold up the session start.
- `agents` never overwrites a list it cannot read. Every action then fails.
- `sentry-test` fails with an error when error reports are off.

## See also

- [[_index]]
- [[Crate Dependency Flow]]

---
tags: [core, codewiki-mcp]
---

# codewiki-mcp

> MCP stdio bridge between coding agents and the CodeWiki backend.

## What it does

Standard output carries the MCP protocol, so nothing else may be printed there; diagnostics go to standard error.

## Source

- `crates/codewiki-mcp/Cargo.toml` — package manifest
- `crates/codewiki-mcp/src/main.rs` — crate root

## How it works

Builds a binary.

1. The program starts at `main`. `main` calls `init` in [[cw-telemetry]].

## Items

Defined at the crate root:

- `fn connected(slot: &Mutex<Option<Backend>>) -> Result<Backend, String>` — the backend in `slot`, connecting on first use. Only a success is kept: after a failed start the next tool call tries again, where a kept error disabled the bridge until the agent restarted it. A kept backend that no longer answers is dropped and a new one found: the app stops the backend when it installs an update, and the bridge outlives that. The lock is held while connecting, so two calls at once never start two backends.
- `fn failed(message: String) -> McpError` — A tool call that failed: logged, and returned to the agent as an error.
- `fn field<'a>(value: &'a Value, key: &str) -> &'a str` — the string under `key`, or an empty string.
- `fn repo_contracts_text(found: &Value) -> String` — text for the contracts of one repository.
- `fn workspace_contracts_text(all: &Value) -> String` — text for contracts matched across the workspace.
- `fn waiting_line(waiting: &Value) -> String` — how many notes wait for an agent, in words; empty when none does.
- `fn sync_text(report: &Value) -> String` — what a sync did, and what it left for an agent.
- `fn waiting_text(queue: &Value) -> String` — the queue of notes that wait, for an agent to read.
- `fn brief_text(brief: &Value) -> String` — the brief of one note, for an agent to read.
- `fn outcome_text(outcome: &Value) -> String` — what became of a submission, for an agent to act on.
- `fn note_text(content: &Value) -> String` — the text of a note as the backend answered, with a last line that names what was left out and how to get it.
- `async fn main() -> std::process::ExitCode` — private function, returns `std::process::ExitCode`.

And 22 more private items with no comment.

## Depends on

- [[Backend]] — uses `Backend`, `answers`, `connect`, `repo_id`
- [[cw-client]] — uses `encode`
- [[cw-core]] — uses `MCP_CLIENT`
- [[cw-telemetry]] — uses `init`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

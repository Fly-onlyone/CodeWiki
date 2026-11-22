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

Modules:

- [[Guard Module]] — which tool calls coding agents may make: folders closed to them in CodeWiki's settings get no tools, and cross-project tools do not show listed projects.

Defined at the crate root:

- `fn connected(slot: &Mutex<Option<Backend>>) -> Result<Backend, String>` — the backend in `slot`, connecting on first use. Only a success is kept: after a failed start the next tool call tries again, where a kept error disabled the bridge until the agent restarted it. A kept backend that no longer answers is dropped and a new one found: the app stops the backend when it installs an update, and the bridge outlives that. The lock is held while connecting, so two calls at once never start two backends.
- `fn unchecked(error: &str) -> String` — why a call that must check the repository it reaches cannot go ahead.
- `const REFUSED: &str` — marks an error that is a refusal, found once the backend has answered: `answered` gives the agent its text as a normal result.
- `fn refused(text: impl Into<String>) -> McpError` — A refusal that a tool's work returns as an error, to stop it there.
- `fn answered(response: Result<CallToolResponse, McpError>) -> Result<CallToolResponse, McpError>` — A tool call's response, with a refusal made the text an agent reads, as the refusals of `screen` are.
- `fn listed_folders() -> Result<Vec<PathBuf>, McpError>` — the folders on the list. A list that cannot be read refuses the call, as `screen` does: it never passes for an empty one.
- `visible_repos` — the `/v1/repos` listing without the repositories whose folders `entries` keep or that hold one of them, and their names.
- `get_visible` — fetch `uri` and drop the repositories `entries` keep from it with `hide`. With an empty list the answer is returned as it is.
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

- [[Guard Module]] — uses `direct_path`, `hide_in_contracts`, `hide_in_workspace`, `hide_repos`
- [[Agents Off Module]] — uses `blocking`, `file`, `load`, `repo_refusal`
- [[Backend]] — uses `Backend`, `answers`, `connect`, `repo_id`
- [[cw-client]] — uses `encode`
- [[cw-core]] — uses `MCP_CLIENT`
- [[cw-telemetry]] — uses `init`
- [[Paths Module]] — uses `data_dir`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

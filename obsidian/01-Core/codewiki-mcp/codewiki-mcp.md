---
tags: [core, codewiki-mcp]
---

# codewiki-mcp

> MCP stdio bridge between coding agents and the CodeWiki backend.

## What it does

`codewiki-mcp` is the program a coding agent starts to use CodeWiki. It speaks the Model Context Protocol (MCP) over standard input and output. It holds no data of its own. Each tool the agent calls, such as `get_context`, `search_codebase` or `submit_note`, becomes an HTTP call to the CodeWiki backend through [[cw-client]]. The answer is turned into text an agent reads easily.

Standard output carries the protocol, so nothing else may be printed there. Logs go to the log file and to standard error.

## Source

- `crates/codewiki-mcp/Cargo.toml` — package manifest
- `crates/codewiki-mcp/src/main.rs` — crate root

## How it works

`main` starts [[cw-telemetry]], builds one `Bridge` and serves it on stdio until the agent closes the connection.

The bridge connects to the backend on the first tool call, through `connected`. It keeps only a success. A failed start is tried again on the next call. A backend that stops answering, for example because the app stopped it to install an update, is dropped and found again.

Each tool is a method on `Bridge`. It runs its work on a blocking task through `call`, which finds the repository (by id, name or path, defaulting to the agent's working folder), calls the backend, and returns JSON. Tools that span repositories use `call_global` instead. The tool then formats that JSON, for example `waiting_text`, `brief_text` and `outcome_text` for the note-writing tools.

Before any tool runs, `call_tool` runs `screen`. It reads the list of folders the user kept from coding agents, in the [[Agents Off Module]]. A call for a closed folder gets a refusal text as a normal result, and no backend is started. `call` checks again once the repository id is known. Tools that span projects, such as `list_repos`, hide the listed projects through the [[Guard Module]].

When an agent lists waiting notes with a claim, the bridge gives that listing an agent id, built by `ClaimIds`. A submission then sends the id its note was listed under. A note that a sync has removed meanwhile gets a plain "No such note, skip it" answer (`no_such_note`), not an error.

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
- `fn is_no_such_note(error: &McpError) -> bool` — whether a call failed because the wiki has no note of the name given.
- `fn no_such_note(note: &str) -> String` — what an agent is told of a note that is not in the wiki. It is no failure of the agent's: a sync removes a note when what it describes is gone from the repository, and a list of waiting notes may be older than that sync.
- `fn field<'a>(value: &'a Value, key: &str) -> &'a str` — the string under `key`, or an empty string.
- `struct ClaimIds` — the agent ids this bridge lists notes under. Every listing that claims gets an id of its own: the main agent, its background note writer and a note writer launched after `/clear` all run through one bridge, and with one id for all of them a second batch was handed the notes the first still held. A submission sends the id its note was listed under.
- `held_key` — the key of the note that `note` names in `repo`, read as the backend reads it: a vault path, or a note's name, which is its file name without `.md` in any case.
- `fn agent_id() -> &'static str` — names this bridge process to the backend, so it can keep notes for it. Made once. A listing that claims adds a count to it (`ClaimIds::next`); a note not claimed through this bridge is submitted under it alone.
- `fn waiting_line(waiting: &Value) -> String` — how many notes wait for an agent, in words; empty when none does.
- `fn sync_text(report: &Value) -> String` — what a sync did, and what it left for an agent.
- `fn waiting_text(queue: &Value, claimed: bool) -> String` — the queue of notes that wait, for an agent to read. `claimed` tells that the listing asked to keep the notes for this agent.
- `fn note_text(content: &Value) -> String` — the text of a note as the backend answered, with a last line that names what was left out and how to get it.
- `async fn main() -> std::process::ExitCode` — private function, returns `std::process::ExitCode`.

And 26 more items.

## Depends on

- [[Guard Module]] — uses `direct_path`, `hide_in_contracts`, `hide_in_workspace`, `hide_repos`
- [[Agents Off Module]] — uses `blocking`, `file`, `load`, `repo_refusal`
- [[Backend]] — uses `Backend`, `answers`, `connect`, `repo_id`
- [[cw-core]] — uses `CLAIM_MINUTES`, `MCP_CLIENT`
- [[cw-client]] — uses `encode`
- [[cw-telemetry]] — uses `init`
- [[Paths Module]] — uses `data_dir`

## Gotchas

- Never print to standard output. It would corrupt the protocol.
- A list of kept folders that cannot be read refuses the call. It never counts as an empty list.
- Refusals reach the agent as ordinary text, not as errors. `answered` converts the marked error back to text.
- A repository given by name or relative path is turned into an id or an absolute path before the backend sees it, so the backend never reads it against its own folder.
- Each claiming listing gets its own id. One id for all would hand a second batch the notes the first still holds, when a main agent and its background writers share one bridge.
- `search_codebase` and `get_refactoring_plan` clamp `limit` to 1 to 50, and `list_waiting_notes` caps the batch it asks for.
- `get_architecture` reads at most six flow notes.
- The `cw-client` dependency carries a version next to its path, so it must follow the workspace version.

## See also

- [[_index]]
- [[Crate Dependency Flow]]

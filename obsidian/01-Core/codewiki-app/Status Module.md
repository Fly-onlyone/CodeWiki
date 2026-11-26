---
tags: [core, codewiki-app, module]
---

# Status Module

> What the app says about itself in the tray: what the backend is doing and whether an agent is using it.

## What it does

The backend's answer to `GET /v1/status` is read here, and put into the words of the tooltip and of the tray menu's first rows. Everything in this module is a function of its arguments, so that the words can be tested without a backend or a tray.

## Source

- `crates/codewiki-app/src/status.rs` — module `codewiki_app::status`

## How it works

`parse` reads the body of `GET /v1/status` into a `Backend`: repositories being updated, how long ago an agent's MCP bridge last called, the embedding job text, proposal counts per repository, and a notes revision number. Ages are computed from the backend's own `now`, so a backend on another machine with a different clock gives the same result.

The tray words come from small functions. `state_text` says what the app is doing: its own busy label first, then Backend not running, then Idle or Updating wiki (naming the repository when it is not the open one). `state_with_job` swaps Idle for the embedding job text. `mcp_text` says active when a bridge called within five minutes, else idle with the age from `ago`.

`tooltip` joins app name, repository, state and MCP into one line. When it is too long, only the repository name is shortened, with an ellipsis, so state and MCP stay readable. `cut_utf16` cuts by UTF-16 units and never inside a character.

`rises` compares two proposal counts and lists repositories whose count went up; [[Review Module]] uses it to decide what is news.

## Items

- `const ACTIVE_FOR: u64` — an agent's MCP bridge that called within this many seconds is "active".
- `pub const TOOLTIP_UNITS: usize` — the longest tooltip Windows shows, in UTF-16 units: the field holds 128 with its end mark.
- `pub struct Backend` — what the backend last said it is doing.
- `pub enum Bridge` — when an agent's MCP bridge last called the backend.
- `pub fn parse(body: &Value) -> Backend` — read the body of `GET /v1/status`. Times are ages by the backend's own clock (`now`), so that a backend on another machine with another clock reads the same.
- `rises` — the repositories whose count of waiting proposals went up from `before` to `after`: each with its id, its new count and how much it rose, by id. Only a repository in both counts; with no `before`, which is the first answer, there is nothing to compare and so no rise.
- `fn model_job_text(job: &Value) -> Option<String>` — what the embedding job is doing, in a few words; `None` when none runs. An older backend says nothing of it.
- `pub fn state_with_job(state: String, model_job: Option<&str>) -> String` — the state the tray shows: what `state_text` says, except that a backend that would be idle but for its work on a model says that work.
- `fn ago(seconds: u64) -> String` — an age in the largest unit that fits: "just now", "12 min ago", "3 h ago", "2 days ago".
- `pub fn mcp_text(bridge: Bridge) -> String` — what follows "MCP:" in the tooltip and the menu.
- `state_text` — what the app and its backend are doing, in a few words. `busy` is what the app itself is waiting for, as its toolbar says it. `updating` names the repositories whose wiki the backend is updating: the open one is `current`, any other is named.
- `pub fn tooltip(repo: Option<&str>, state: &str, mcp: &str) -> String` — the tooltip of the tray icon: `CodeWiki - <repo> - <state> - MCP: <mcp>`, at most `TOOLTIP_UNITS` long, with `CodeWiki Dev` in a development build (`cw_core::APP_NAME`). The repository's name comes first and is what gets shortened when the whole does not fit, so that the state and the MCP status stay readable.
- `pub fn cut_utf16(text: &str, units: usize) -> String` — the start of `text` that fits in `units` UTF-16 units, never ending in the middle of a character.

## Depends on

- [[cw-core]] — uses `APP_NAME`

## Used by

- [[codewiki-app]] — from `AppView`, `take_status`, `tray_snapshot`
- [[Review Module]] — from `news`
- [[Tray Module]] — from `tooltip`

## Gotchas

- The tooltip limit is 127 UTF-16 units, not characters or bytes. Windows holds 128 with its end mark.
- With no earlier count, `rises` returns nothing, so the first answer never counts as news. A repository present in only one of the two counts is also skipped.
- An older backend may omit proposals, bridge or job fields. Each maps to a safe default (unknown, no counts).
- The proposal counts can lack repositories the backend did not count in time on slow storage.

## See also

- [[_index]]
- [[codewiki-app]]

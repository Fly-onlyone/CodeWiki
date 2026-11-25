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

1. `AppView::take_status` in [[codewiki-app]] calls `parse`. `parse` calls `model_job_text`.
2. `AppView::tray_snapshot` in [[codewiki-app]] calls `state_with_job`.
3. `AppView::tray_snapshot` in [[codewiki-app]] calls `mcp_text`. `mcp_text` calls `ago`.
4. `AppView::tray_snapshot` in [[codewiki-app]] calls `state_text`.
5. `tooltip` in [[Tray Module]] calls `tooltip`. `tooltip` calls `cut_utf16`.

## Items

- `const ACTIVE_FOR: u64` — an agent's MCP bridge that called within this many seconds is "active".
- `pub const TOOLTIP_UNITS: usize` — the longest tooltip Windows shows, in UTF-16 units: the field holds 128 with its end mark.
- `pub struct Backend` — what the backend last said it is doing.
- `pub enum Bridge` — when an agent's MCP bridge last called the backend.
- `pub fn parse(body: &Value) -> Backend` — read the body of `GET /v1/status`. Times are ages by the backend's own clock (`now`), so that a backend on another machine with another clock reads the same.
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
- [[Tray Module]] — from `tooltip`

## See also

- [[_index]]
- [[codewiki-app]]

---
tags: [core, codewiki-app, module]
---

# Status Module

> What the app says about itself in the tray: what the backend is doing and whether an agent is using it.

## Source

- `crates/codewiki-app/src/status.rs` — module `codewiki_app::status`

## How it works

Main items:

- `Bridge` — when an agent's MCP bridge last called the backend.
- `cut_utf16` — the start of `text` that fits in `units` UTF-16 units, never ending in the middle of a character.
- `tooltip` — the tooltip of the tray icon: `CodeWiki - <repo> - <state> - MCP: <mcp>`, at most `TOOLTIP_UNITS` long.
- `Backend` — what the backend last said it is doing.
- `TOOLTIP_UNITS` — the longest tooltip Windows shows, in UTF-16 units: the field holds 128 with its end mark.
- `parse` — read the body of `GET /v1/status`.
- `mcp_text` — what follows "MCP:" in the tooltip and the menu.
- `state_text` — what the app and its backend are doing, in a few words.

## Used by

- [[codewiki-app]] — from `AppView`, `take_status`, `tray_snapshot`
- [[Tray Module]] — from `tooltip`

## See also

- [[_index]]
- [[codewiki-app]]

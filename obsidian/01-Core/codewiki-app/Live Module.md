---
tags: [core, codewiki-app, module]
---

# Live Module

> The live pages: one symbol, one file, search results, hotspots, the theme picker, a proposal, the result of a sync, and Home, the page the app starts on.

## Source

- `crates/codewiki-app/src/live.rs` — module `codewiki_app::live`

## How it works

Like the reports in `pages`, each turns what the backend sent into a `Doc`. These show code that the vault deliberately does not hold as notes.

Main items:

- `Project` — A registered repository, as Home lists it.
- `message_page` — A page that only says something, such as that there is no wiki yet.
- `proposal_page` — one proposal for review.
- `hotspots_page` — the hotspot ranking.
- `sync_page` — what a sync did.
- `search_page` — search results, grouped by what was found.
- `symbol_page` — one symbol: what it is, where it is, and what it touches.
- `file_page` — one file: risk, symbols, relations and notes.

## Depends on

- [[Doc Module]] — uses `Line`, `Row`, `code`, `detail`
- [[Doc]] — uses `Doc`, `heading`, `lead`, `new`

## Used by

- [[codewiki-app]] — from `connect`, `home_doc`, `listed`, `open_file`

## See also

- [[_index]]
- [[codewiki-app]]

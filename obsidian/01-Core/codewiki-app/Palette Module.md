---
tags: [core, codewiki-app, module]
---

# Palette Module

> The command palette's entries and matching.

## Source

- `crates/codewiki-app/src/palette.rs` — module `codewiki_app::palette`

## How it works

No UI here.

Main items:

- `Command` — what running a palette entry does.
- `Entry` — struct
- `actions` — the fixed commands, in the order shown for an empty query.
- `filter` — entries matching `query`, best first, at most `limit`.
- `rank` — how well `label` matches the query words; lower is better, `None` is no match.

## Used by

- [[codewiki-app]] — from `AppView`, `palette_matches`, `run_command`

## See also

- [[_index]]
- [[codewiki-app]]

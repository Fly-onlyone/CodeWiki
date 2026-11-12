---
tags: [core, cw-vault, module]
---

# Lint Module

> Style linter for vault notes.

## Source

- `crates/cw-vault/src/lint.rs` — module `cw_vault::lint`

## How it works

Main items:

- `Severity` — enum
- `Issue` — struct
- `MAX_WORDS` — const
- `MIN_WORDS` — const
- `Mode` — enum
- `link_resolution_rate` — share of wikilinks across `notes` that resolve, from 0.0 to 1.0.
- `lint` — lint one note.
- `has_inline_tag` — `word` at a word boundary, where `word` starts with a letter.

## Depends on

- [[Note]] — uses `parse`, `section`
- [[NoteKind]] — uses `NoteKind`, `sections`
- [[Registry]] — uses `Registry`, `resolve`
- [[Note Module]] — uses `extract_links`

## Used by

- [[Generate Module]] — from `generate`

## See also

- [[_index]]
- [[cw-vault]]

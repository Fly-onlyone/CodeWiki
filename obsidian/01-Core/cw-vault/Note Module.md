---
tags: [core, cw-vault, module]
---

# Note Module

> Parsing of a note's structure: frontmatter, title, summary, sections, links.

## Source

- `crates/cw-vault/src/note.rs` — module `cw_vault::note`

## How it works

Main items:

- [[NoteKind]] — what a note documents.
- [[Note]] — struct
- `Section` — struct
- `WikiLink` — struct
- `extract_links` — find `(target)`, `(target|alias)`, `(target#heading)` and `!(embed)`.
- `count_words` — function
- `parse_tags` — read `tags: a, b` or a YAML block list under `tags:`.
- `push_line` — function

## Used by

- [[Note]] — from `Note`, `parse`, `section`
- [[Lint Module]] — from `lint`

## See also

- [[_index]]
- [[cw-vault]]

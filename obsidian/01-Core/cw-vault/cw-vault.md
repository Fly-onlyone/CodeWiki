---
tags: [core, cw-vault]
---

# cw-vault

> Obsidian vault model for CodeWiki: notes, manifest, name registry, linter, lifecycle and proposals.

## Source

- `crates/cw-vault/Cargo.toml` — package manifest
- `crates/cw-vault/src/lib.rs` — crate root

## How it works

Builds a library.

Modules:

- [[Lifecycle Module]] — note lifecycle: what may be done to a note given its state on disk.
- [[Lint Module]] — style linter for vault notes.
- [[Manifest Module]] — machine metadata stored in `obsidian/.codewiki/`.
- [[Note Module]] — parsing of a note's structure: frontmatter, title, summary, sections, links.
- [[Proposal Module]] — the review queue: proposed changes to human-edited notes.
- [[Registry Module]] — the note-name registry.

## Depends on

- [[Error Module]] — uses `Result`
- [[Error]] — uses `external`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

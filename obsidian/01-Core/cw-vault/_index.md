---
tags: [moc, core, cw-vault]
---

# Core / cw-vault — Map of Content

> Notes for `cw-vault`: 14 concepts.

## Notes

- [[cw-vault]] — Obsidian vault model for CodeWiki: notes, manifest, name registry, linter, lifecycle and proposals.
- [[Explanation Module]] — who writes which part of a note, and how the two parts are put together.
- [[Lifecycle Module]] — note lifecycle: what may be done to a note given its state on disk.
- [[Lint Module]] — style linter for vault notes.
- [[Manifest Module]] — machine metadata stored in `obsidian/.codewiki/`.
- [[Note Module]] — parsing of a note's structure: frontmatter, title, summary, sections, links.
- [[NoteEntry]] — `NoteEntry` struct defined in `cw_vault::manifest`.
- [[NoteKind]] — what a note documents.
- [[Proposal Module]] — the review queue: proposed changes to human-edited notes.
- [[Registry]] — maps note names (file stem, compared case-insensitively because Windows file names and Obsidian links are) to entries.
- [[Registry Module]] — the note-name registry.
- [[Section]] — `Section` struct defined in `cw_vault::note`.
- [[SourceRef]] — A source file a note describes, with the content hash seen at generation.
- [[VaultMeta]] — everything under `.codewiki/`, loaded.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]

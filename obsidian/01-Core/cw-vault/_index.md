---
tags: [moc, core, cw-vault]
---

# Core / cw-vault — Map of Content

> Notes for `cw-vault`: 12 concepts.

## Notes

- [[cw-vault]] — Obsidian vault model for CodeWiki: notes, manifest, name registry, linter, lifecycle and proposals.
- [[Lifecycle Module]] — note lifecycle: what may be done to a note given its state on disk.
- [[Lint Module]] — style linter for vault notes.
- [[Manifest Module]] — machine metadata stored in `obsidian/.codewiki/`.
- [[Note]] — `Note` struct defined in `cw_vault::note` with 2 methods.
- [[Note Module]] — parsing of a note's structure: frontmatter, title, summary, sections, links.
- [[NoteKind]] — what a note documents.
- [[Proposal Module]] — the review queue: proposed changes to human-edited notes.
- [[Registry]] — maps note names (file stem, compared case-insensitively because Windows file names and Obsidian links are) to entries.
- [[Registry Module]] — the note-name registry.
- [[RegistryEntry]] — `RegistryEntry` struct defined in `cw_vault::registry`.
- [[VaultMeta]] — everything under `.codewiki/`, loaded.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]

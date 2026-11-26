---
tags: [core, cw-vault]
---

# cw-vault

> Obsidian vault model for CodeWiki: notes, manifest, name registry, linter, lifecycle and proposals.

## What it does

The model of an Obsidian vault for CodeWiki. It knows what a note is and how to read one, which sections each kind of note has, who wrote which part, which notes exist and what they are called, whether a note may be rewritten, and how changes to human-edited notes wait for review. Other crates generate and serve notes; this one decides what is allowed and keeps the records.

## Source

- `crates/cw-vault/Cargo.toml` — package manifest
- `crates/cw-vault/src/lib.rs` — crate root

## How it works

The crate is a library with seven parts, each in its own module:

1. [[Note Module]] parses a note into title, summary, sections and links, and defines [[NoteKind]].
2. [[Manifest Module]] stores the bookkeeping for every note, loaded as [[VaultMeta]].
3. [[Registry Module]] holds the unique note names and resolves links.
4. [[Lint Module]] checks a note's style.
5. [[Lifecycle Module]] turns a note's state into an action: keep, regenerate, merge or propose.
6. [[Proposal Module]] stores proposals for notes a person edited.
7. [[Explanation Module]] splits a note between the rules' sections and an agent's sections and weaves them back together.

A typical update asks `state` and `action` about each note, writes or merges the note if allowed, runs the linter, and records the result in the manifest.

## Items

Modules:

- [[Explanation Module]] — who writes which part of a note, and how the two parts are put together.
- [[Lifecycle Module]] — note lifecycle: what may be done to a note given its state on disk.
- [[Lint Module]] — style linter for vault notes.
- [[Manifest Module]] — machine metadata stored in `obsidian/.codewiki/`.
- [[Note Module]] — parsing of a note's structure: frontmatter, title, summary, sections, links.
- [[Proposal Module]] — the review queue: proposed changes to human-edited notes.
- [[Registry Module]] — the note-name registry.

## Depends on

- [[Error Module]] — uses `Result`
- [[Error]] — uses `external`

## Gotchas

- This crate does not generate note text or call a model. It only decides, parses, checks and records.
- Rules and agents share one note, divided by section. A person who edits any part makes the whole note human-owned, and later changes go through the proposal queue.
- Metadata sits in plain sorted files so that git diffs stay small. Hand edits to them change how CodeWiki sees a note.

## See also

- [[_index]]
- [[Crate Dependency Flow]]

---
tags: [core, cw-vault]
---

# NoteKind

> What a note documents.

## What it does

Decides its section template and its zone.

## Source

- `crates/cw-vault/src/note.rs` — definition of `NoteKind`

## How it works

1. `adopt_existing` and `adopted_kind` in [[Generate Module]] call `from_file_name`.

## Items

Declared as `pub enum NoteKind`.

Members:

- `pub fn sections(self) -> &'static [&'static str]` — section headings in required order. Empty sections are omitted by the writer, so every heading is optional but order is fixed.
- `pub fn from_file_name(name: &str) -> Option<Self>` — kind implied by a note's file name, for map notes.

## Used by

- [[Lint Module]] — from `lint`, `lint_lines`, `max_words`
- [[codewiki]] — from `has_template`, `note_style_skill_follows_the_lint`
- [[Writer Module]] — from `guidance`, `kind_hint`
- [[Generate Module]] — from `adopt_existing`, `adopted_kind`
- [[Query Module]] — from `NoteInfo`, `agent_view`
- [[Manifest Module]] — from `NoteEntry`
- [[Registry]] — from `reserve`
- [[RegistryEntry]] — from `RegistryEntry`

## See also

- [[_index]]
- [[Note Module]]

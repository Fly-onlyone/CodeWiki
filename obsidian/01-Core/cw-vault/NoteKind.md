---
tags: [core, cw-vault]
---

# NoteKind

> What a note documents.

## Source

- `crates/cw-vault/src/note.rs` — definition of `NoteKind`

## How it works

Declared as `pub enum NoteKind`.

Decides its section template and its zone.

Public API:

- `pub fn from_file_name(name: &str) -> Option<Self>` — kind implied by a note's file name, for map notes.
- `pub fn sections(self) -> &'static [&'static str]` — section headings in required order.

## Used by

- [[Lint Module]] — from `lint`, `lint_lines`
- [[Generate Module]] — from `adopt_existing`, `adopted_kind`
- [[Writer Module]] — from `kind_hint`
- [[Manifest Module]] — from `NoteEntry`
- [[Query Module]] — from `NoteInfo`
- [[Registry]] — from `reserve`
- [[RegistryEntry]] — from `RegistryEntry`
- [[Planned]] — from `Planned`

## See also

- [[_index]]
- [[Note Module]]

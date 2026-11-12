---
tags: [core, cw-vault]
---

# Registry

> Maps note names (file stem, compared case-insensitively because Windows file names and Obsidian links are) to entries.

## Source

- `crates/cw-vault/src/registry.rs` — definition of `Registry`

## How it works

Declared as `pub struct Registry`.

Map notes (`_index`, `_HOME`) are keyed by path instead, because every folder has one.

Public API:

- `pub fn entries(&self) -> impl Iterator<Item = &RegistryEntry>` — all entries, named notes first, each group in path order.
- `pub fn insert(&mut self, entry: RegistryEntry) -> Result<()>` — register a note.
- `pub fn resolve(&self, target: &str, from_path: &str) -> Option<&RegistryEntry>` — resolve a wikilink target written in the note at `from_path`.
- `pub fn reserve(&mut self, id: &str, dir: &str, title: &str, kind: NoteKind) -> RegistryEntry` — reserve a unique name for `title` in `folder`, appending the folder's last segment and then a counter when the plain title is taken.
- `pub fn by_id(&self, id: &str) -> Option<&RegistryEntry>`
- `pub fn len(&self) -> usize`

## Depends on

- [[Registry Module]] — uses `folder`, `join`, `sanitize_name`, `stem`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`
- [[NoteKind]] — uses `NoteKind`
- [[RegistryEntry]] — uses `RegistryEntry`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `link`, `name`, `new`
- [[Lint Module]] — from `link_resolution_rate`, `lint`
- [[VaultMeta]] — from `registry`, `save`
- [[Generate Module]] — from `generate`

## See also

- [[_index]]
- [[Registry Module]]

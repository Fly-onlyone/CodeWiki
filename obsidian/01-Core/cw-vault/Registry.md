---
tags: [core, cw-vault]
---

# Registry

> Maps note names (file stem, compared case-insensitively because Windows file names and Obsidian links are) to entries.

## What it does

Map notes (`_index`, `_HOME`) are keyed by path instead, because every folder has one.

## Source

- `crates/cw-vault/src/registry.rs` — definition of `Registry`

## How it works

1. `generate_with` in [[Generate Module]] and `VaultMeta::registry` in [[VaultMeta]] call `new`.

## Items

Declared as `pub struct Registry`.

Members:

- `pub fn new() -> Self` — public method, returns `Self`; called by `VaultMeta::registry`, `generate_with`.
- `pub fn insert(&mut self, entry: RegistryEntry) -> Result<()>` — register a note. Fails when another note already owns the name.
- `pub fn reserve(&mut self, id: &str, dir: &str, title: &str, kind: NoteKind) -> RegistryEntry` — reserve a unique name for `title` in `folder`, appending the folder's last segment and then a counter when the plain title is taken.
- `pub fn by_name(&self, name: &str) -> Option<&RegistryEntry>` — public method, returns `Option<&RegistryEntry>`.
- `pub fn by_id(&self, id: &str) -> Option<&RegistryEntry>` — public method, returns `Option<&RegistryEntry>`.
- `pub fn contains_name(&self, name: &str) -> bool` — public method, returns `bool`.
- `pub fn entries(&self) -> impl Iterator<Item = &RegistryEntry>` — all entries, named notes first, each group in path order.
- `pub fn len(&self) -> usize` — public method, returns `usize`; called by `Registry::is_empty`.
- `pub fn is_empty(&self) -> bool` — public method, returns `bool`; called by `Registry::reserve`.
- `pub fn resolve(&self, target: &str, from_path: &str) -> Option<&RegistryEntry>` — resolve a wikilink target written in the note at `from_path`.

## Depends on

- [[Registry Module]] — uses `RegistryEntry`, `folder`, `join`, `sanitize_name`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`
- [[NoteKind]] — uses `NoteKind`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `link`, `name`, `new`
- [[Lint Module]] — from `link_resolution_rate`, `lint`
- [[Generate Module]] — from `Links`, `generate_with`
- [[VaultMeta]] — from `registry`, `save`

## See also

- [[_index]]
- [[Registry Module]]

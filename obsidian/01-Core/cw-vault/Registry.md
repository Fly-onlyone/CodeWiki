---
tags: [core, cw-vault]
---

# Registry

> Maps note names (file stem, compared case-insensitively because Windows file names and Obsidian links are) to entries.

## What it does

The table of all note names in a vault. It answers one question: which note does this link target mean? Ordinary notes are keyed by file stem, compared in lower case. Map notes (`_index` and `_HOME`) are keyed by their path instead, since every folder has an `_index`. The linter and the generator both use it, so a link that does not resolve here is reported as an error.

## Source

- `crates/cw-vault/src/registry.rs` — definition of `Registry`

## How it works

`insert` adds an entry. A second ordinary note with the same lower-case stem and another path is an error. `reserve` picks a free name for a new title: the plain title, then the title with the folder's last segment in parentheses, then the title with a counter (2, 3, and so on). It records the choice at once, so later calls see it.

`resolve` takes a link target and the path of the note that holds the link:

- A target with a slash is read as a vault path.
- `_index` means the map of the note's own folder. From a map note it means the parent folder's map.
- `_HOME` means the root map.
- Anything else is looked up by name, ignoring case.

`entries` lists named notes first, then maps, each in path order. `VaultMeta::registry` in [[VaultMeta]] builds the registry from the note entries.

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

- [[Registry Module]] — uses `folder`, `join`, `sanitize_name`, `stem`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`
- [[NoteKind]] — uses `NoteKind`
- [[RegistryEntry]] — uses `RegistryEntry`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `link`, `name`, `new`
- [[Lint Module]] — from `link_resolution_rate`, `lint`
- [[Generate Module]] — from `Links`, `generate_with`
- [[VaultMeta]] — from `registry`, `save`

## Gotchas

- A bare `_index` link in a note outside any folder resolves to nothing.
- Lookups ignore case, but the stored path keeps its case.
- `reserve` changes the registry even if the caller later drops the note.
- The registry is saved as `registry.json`, but it is rebuilt from the note entries on every save. Editing the file by hand has no lasting effect.

## See also

- [[_index]]
- [[Registry Module]]

---
tags: [core, cw-wiki]
---

# Tally

> Files, lines, folders and languages of a set of source files.

## Source

- `crates/cw-wiki/src/filemap.rs` — definition of `Tally`

## How it works

What the functions of this type call:

1. `language_names` calls `names_up_to`. `names_up_to` calls `languages`, then `count`, then `join_and` (both in [[Render Module]]).

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct Tally`.

Members:

- `pub fn folders(&self) -> usize` — folders that hold a file.
- `pub fn languages(&self) -> Vec<(&'static str, usize)>` — languages with their file counts, the largest first.
- `pub fn language_names(&self) -> String` — `C#`, `C# and AXAML`, `Lua, Ruby, SQL, and 2 other languages`.
- `fn names_up_to(&self, limit: usize) -> String` — the names of up to `limit` languages, the rest counted.

And 1 more private item with no comment.

## Depends on

- [[Filemap Module]] — uses `MAX_LANGUAGES_NAMED`, `SourceFile`, `parent_dir`
- [[Render Module]] — uses `count`, `join_and`

## Used by

- [[Filemap Module]] — from `FileMap`, `FileUnit`, `Folder`, `build`

## See also

- [[_index]]
- [[Filemap Module]]

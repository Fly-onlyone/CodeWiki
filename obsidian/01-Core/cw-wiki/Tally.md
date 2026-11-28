---
tags: [core, cw-wiki]
---

# Tally

> Files, lines, folders and languages of a set of source files.

## What it does

A running total for a set of source files: how many files, how many lines, which folders hold a file, and how many files of each language. [[Filemap Module]] keeps one for the whole repository, one for each unit and one for each folder note, and builds its summary sentences from them.

## Source

- `crates/cw-wiki/src/filemap.rs` — definition of `Tally`

## How it works

`add` is private and counts one file: it adds one to the files, adds its lines, records the file's folder in a set, and adds one to its language. `folders` returns the size of that set. `languages` sorts the counts largest first, ties by name. `language_names` writes the readable list through `names_up_to`: at most three names, then "and N other languages", for example `Lua, Ruby, SQL, and 2 other languages`.

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

## Gotchas

- Only [[Filemap Module]] can add files, because `add` is private.
- A folder counts once however many files it holds.
- The folder set is keyed by each file's own folder, not its parents.

## See also

- [[_index]]
- [[Filemap Module]]

---
tags: [core, cw-wiki, module]
---

# Filemap Module

> The file-level map: units and folders for source files of a language CodeWiki has no parser for.

## What it does

A file counts when `Language::file_level_name` names its language. Such a file has a folder, a language and a size in lines, and nothing else is known about it: no symbols, no imports, no calls. What is said about it is therefore always correct, whatever the language. A language that gets a parser leaves this map, because `Language::file_level_name` stops naming it; no list of languages lives here.

Units:

A unit is the counterpart of a crate or a package, and has a note of its own under `01-Core`. The rule, from the paths alone:

- A file at `<container>/<project>/…`, where `<container>` is one of `CONTAINERS` (`src`, `tests`, `apps`, `packages`, …), belongs to the unit `<container>/<project>`. A layout of `src/<Project>/` and `tests/<Project>/` therefore gives one unit per project.
- Every other file belongs to the root unit, the repository itself. A flat repository is one unit.
- A unit of fewer than `MIN_UNIT_FILES` files is folded into the root unit, and beside mapped code (a repository with parsed units) the root unit must hold that many files too, so that a few scripts or style sheets beside a Rust workspace get no notes. A unit an agent explained is never dropped while it has a file.

Folders:

A folder gets a note when the files it holds, its own and those of the folders below it that have no note, number at least `MIN_FOLDER_FILES`. A smaller folder rolls up: its files are listed in the nearest folder above it that has a note, or in the unit's note. At most `MAX_FOLDER_NOTES` folders get a note, the largest in lines first; the others roll up the same way. The folder of a note an agent explained is never cut while it has a file.

Ids are `unit:<name>` and `dir:<repo-relative folder>`. A folder's id names the place, never its neighbours, so no other file changes it.

## Source

- `crates/cw-wiki/src/filemap.rs` — module `cw_wiki::filemap`

## How it works

1. `Tally::add` in [[Tally]] calls `parent_dir`.
2. `plan` in [[Plan Module]] calls `build`. `build` calls `Language::file_level_name` in [[Language]], then `unit_dir`, then `slug` in [[Units Module]], then `unique`, then `candidates`, then `cut` in [[Plan Module]], and 5 more. `candidates` calls `parent_dir`, then `parent_of`, then `depth`.
3. `RenderCtx::summary` in [[RenderCtx]] calls `folder_summary`. `folder_summary` calls `count` in [[Render Module]].
4. `RenderCtx::render` in [[RenderCtx]] calls `unit_sections`. `unit_sections` calls `holds`, then `folder_bullet`, then `file_lines`, then `source_lines`. `holds` calls `count`, then `join_and` (both in [[Render Module]]). `folder_bullet` calls `count` in [[Render Module]].
5. `RenderCtx::render` in [[RenderCtx]] calls `folder_sections`. `folder_sections` calls `holds`, then `folder_bullet`, then `file_lines`, then `source_lines`. `file_lines` calls `count` in [[Render Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const MIN_UNIT_FILES: usize` — fewer files than this and a unit folds into the repository's own.
- `pub const MIN_FOLDER_FILES: usize` — fewer files than this and a folder rolls up into the one above it.
- `pub const MAX_FOLDER_NOTES: usize` — the most folders that get a note, as many as a module ranking allows.
- `pub struct SourceFile` — A source file of a language without a parser.
- [[Tally]] — files, lines, folders and languages of a set of source files.
- `pub struct FileUnit` — the unit of a repository that is not mapped by a parser.
- `pub struct Folder` — A folder with a note.
- `pub struct FileMap` — every unit and folder with a note, and the totals of the repository.
- `fn parent_dir(path: &str) -> &str` — private function, returns `&str`; called by `Tally::add`, `build`, `candidates`.
- `fn parent_of(dir: &str) -> Option<&str>` — the folder above `dir`; `None` for the repository root.
- `build` — build the file-level map of a repository. `taken_slugs` are the lowercase folders under `01-Core` that parsed units already use; `has_mapped` says whether the repository has such units; `pinned` holds the ids of the notes an agent explained.
- `fn file_lines(files: &[SourceFile], base: &str) -> String` — `- `path` — C#, 120 lines`, the paths relative to `base`.
- `fn holds(total: &Tally) -> String` — what a set of files holds: `Holds 3 source files with 60 lines in 1 folder: 2 in AXAML, 1 in C#.`.
- `fn source_lines(files: &[SourceFile]) -> String` — the `Source` section: the files a note cites.
- `pub type Link<'a> = &'a dyn Fn(&str) -> Option<String>` — links by note id; `None` when the note is not in the vault.
- `pub fn unit_summary(unit: &FileUnit) -> String` — One-sentence summary of a unit's note.
- `pub fn folder_summary(map: &FileMap, folder: &Folder) -> String` — One-sentence summary of a folder's note.
- `fn folder_bullet(folder: &Folder, link: Link) -> Option<String>` — private function, returns `Option<String>`; called by `folder_sections`, `unit_sections`.
- `pub fn unit_sections(map: &FileMap, unit: &FileUnit, link: Link) -> Vec<(&'static str, String)>` — the sections rules write for a unit's note, in order.
- `pub fn folder_sections(map: &FileMap, folder: &Folder, link: Link) -> Vec<(&'static str, String)>` — the sections rules write for a folder's note, in order.

And 15 more items.

## Depends on

- [[Tally]] — uses `Tally`, `add`, `folders`, `language_names`
- [[Plan Module]] — uses `ZONE_CORE`, `cut`, `title_case`
- [[Render Module]] — uses `count`, `join_and`
- [[Scan Module]] — uses `RepoFacts`
- [[Units Module]] — uses `slug`
- [[Language]] — uses `file_level_name`

## Used by

- [[Plan Module]] — from `Plan`, `plan`
- [[Tally]] — from `add`, `language_names`
- [[RenderCtx]] — from `render`, `summary`

## See also

- [[_index]]
- [[cw-wiki]]

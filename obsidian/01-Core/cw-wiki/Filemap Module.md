---
tags: [core, cw-wiki, module]
---

# Filemap Module

> The file-level map: units and folders for source files of a language CodeWiki has no parser for.

## What it does

Gives a repository a wiki even where CodeWiki has no parser for its language. For each source file whose language `Language::file_level_name` names, it knows only the folder, the language and the number of lines. So the notes say only what is always true: what a folder holds and how big it is. When a language later gets a parser, it leaves this map by itself, because `file_level_name` stops naming it. No list of languages lives here.

The map has two kinds of note. A unit is the counterpart of a crate or a package. A folder note covers a large enough folder. Ids are `unit:<name>` and `dir:<repo-relative folder>`.

## Source

- `crates/cw-wiki/src/filemap.rs` — module `cw_wiki::filemap`

## How it works

`build` filters the repository's files to the file-level languages and groups them by `unit_dir`. A file at `<container>/<project>/...`, with a container such as `src`, `apps` or `tests`, belongs to the unit `<container>/<project>`. Every other file belongs to the root unit. A unit with fewer than `MIN_UNIT_FILES` files folds into the root unit. Next to parsed code, the root unit must also reach that size, so a few scripts beside a Rust workspace get no notes.

`candidates` judges the folders of a unit from the deepest up. A folder gets a note when its own files, plus those rolled up from folders below that got none, number at least `MIN_FOLDER_FILES`. A smaller folder passes its files to its parent. `build` keeps at most `MAX_FOLDER_NOTES` folders, largest in lines first. Notes an agent explained (the `pinned` ids) are never cut while they still hold a file. `holder` finds the nearest folder with a note for a file. `unit_sections` and `folder_sections` write the note text, and `unit_summary` and `folder_summary` write the one-line summaries. `FileMap::home_summary` writes the sentence on the home note.

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

## Gotchas

- A folder note id names the place only, so a change in a neighbour never renames it.
- Unit slugs must not clash with folders parsed units already use; `unique` adds `-2`, `-3` and so on, ignoring case.
- A note lists at most 40 files and cites at most 10 sources. The rest are only counted.
- A file with no lines read shows its size in bytes, or `empty`.

## See also

- [[_index]]
- [[cw-wiki]]

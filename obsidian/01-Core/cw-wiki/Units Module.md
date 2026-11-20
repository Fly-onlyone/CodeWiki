---
tags: [core, cw-wiki, module]
---

# Units Module

> Units: the top-level parts of a repository, each with its own folder under `01-Core`.

## What it does

A Cargo package is a unit. Code in other languages has no manifest the scanner understands, so its units are derived here from the layout: a Python top-level package, a JavaScript or TypeScript package directory, a Java build module. Every unit has the same shape, so planning and rendering treat them alike.

## Source

- `crates/cw-wiki/src/units.rs` — module `cw_wiki::units`

## How it works

1. `Repo::is_test` in [[Repo]] and `RenderCtx::unit_commands` in [[RenderCtx]] call `is_test_file`.
2. `Repo::at` and `Repo::new` in [[Repo]] call `discover`. `discover` calls `join`, then `Family::of`, then `nearest_with`, then `parent_dir`, then `is_test_file`, then `parse_package_json`, and 3 more. `nearest_with` calls `ancestors`, then `join`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const JS_OUTPUT_DIRS: [&str; 4]` — directories a bundler writes to; `main` in `package.json` often points into one while the source sits under `src`.
- `pub struct Manifest` — A build or dependency file that belongs to a unit.
- `pub struct Entry` — A symbol where execution starts.
- [[Unit]] — one top-level part of the repository.
- [[Units]] — every unit of a repository and the lookup from code to unit.
- `enum Family` — language family: one unit never mixes two of these.
- `pub fn is_test_file(path: &str) -> bool` — true for a test file of a language other than Rust.
- `fn slug(name: &str) -> String` — lowercase name safe as a folder and as a tag.
- `fn parent_dir(path: &str) -> &str` — private function, returns `&str`; called by `discover`.
- `fn join(dir: &str, file: &str) -> String` — private function, returns `String`; called by `discover`, `nearest_with`, `resolve_js_entry`.
- `fn ancestors(dir: &str) -> Vec<&str>` — `dir` and each of its ancestors, nearest first, ending at the root (`""`).
- `fn nearest_with<'a>(files: &HashSet<&str>, dir: &'a str, names: &[&str]) -> Option<&'a str>` — nearest directory at or above `dir` that holds one of `names`.
- `fn resolve_js_entry(files: &HashSet<&str>, dir: &str, spec: &str) -> Option<String>` — source file a `package.json` path names. Published packages point at build output, so the matching file under `src` is tried as well.
- `struct PackageJson` — what a `package.json` says about its package.
- `fn parse_package_json(text: &str) -> PackageJson` — private function, returns `PackageJson`; called by `discover`.
- `struct Group<'a>` — files of one unit that is still being put together.
- `discover` — find the units of a repository. `repo_name` names a unit that is the repository itself; `read` returns the text of a repo-relative file.

And 3 more private items with no comment.

## Depends on

- [[Language]] — uses `Language`, `from_path`
- [[Python Module]] — uses `MAIN_GUARD_ATTR`
- [[Scan Module]] — uses `RepoFacts`
- [[Symbol]] — uses `Symbol`
- [[Unit]] — uses `Unit`
- [[Units]] — uses `Units`

## Used by

- [[Repo]] — from `at`, `is_test`, `new`
- [[Manifests Module]] — from `asked_in`
- [[RenderCtx]] — from `unit_commands`
- [[Unit]] — from `Unit`

## See also

- [[_index]]
- [[cw-wiki]]

---
tags: [core, cw-wiki, module]
---

# Units Module

> Units: the top-level parts of a repository, each with its own folder under `01-Core`.

## What it does

Finds the top-level parts of a repository, called units. Each unit gets its own folder under `01-Core` in the vault. A Cargo package is a unit. For Python, JavaScript, TypeScript and Java the scanner has no manifest it understands, so this module derives units from the layout and the build files. Every unit has the same shape ([[Unit]]), so the planner and the renderer treat Rust and other languages alike.

## Source

- `crates/cw-wiki/src/units.rs` — module `cw_wiki::units`

## How it works

`discover` builds a [[Units]] value in three steps.

1. Each crate from the scan becomes a Rust unit. Its entry is `main` when the crate has a binary.
2. Every non-Rust source file is put into a group by language family. Python files group by their top-level package, and a lone top-level module goes into a shared group. JavaScript and TypeScript files group by the nearest folder up the tree that holds a `package.json`, or by their top folder if there is none. Java files group by the nearest folder with `pom.xml` or a Gradle file, or by the first part of their package name.
3. Each group becomes a unit if it holds at least one real symbol outside test files. The unit's root module is the common prefix of its module names. Its manifests are found next to the group. A `package.json` supplies the name, description, scripts and entries: `main`, `bin` and the file named in the `start` script. `resolve_js_entry` maps a built path such as `dist/x.js` back to a source file under `src`.

`is_test_file` recognises non-Rust tests by folder names (`tests`, `__tests__`, `spec`, `e2e`) and by file patterns such as `test_*.py`, `*.test.ts` and `*Test.java`. `slug` makes a name safe as a folder and a tag.

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

## Gotchas

- A unit never mixes language families. A repository with Python and TypeScript gets separate units.
- Configuration files or test suites alone do not make a unit.
- Unit names and folder slugs are kept unique separately, because a unit's key may equal its folder name.
- Manifests are read through a callback. When it returns nothing (as with `Repo::new`), the unit is still found but its manifest text is empty.

## See also

- [[_index]]
- [[cw-wiki]]

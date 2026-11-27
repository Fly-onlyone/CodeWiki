---
tags: [core, cw-parse, module]
---

# Jsbuild Module

> What the build files of a JavaScript or TypeScript repository say, read as data and never run: `tsconfig.json` and `jsconfig.json` (`baseUrl`, `paths`, `extends`), `package.json` (name, entry points, workspaces, the package manager), `pnpm-workspace.yaml`, and the lock files of npm, pnpm, Yarn and Bun.

## What it does

tsconfig is JSONC: comments and trailing commas are allowed. A bundler's own aliases (`vite.config.ts`, webpack) are code and are not evaluated. The YAML readers are line based and know only the shapes their tests pin; a file in another shape reads as empty, and a lock file of a format this module does not know is "not read", never a guess.

## Source

- `crates/cw-parse/src/jsbuild.rs` — module `cw_parse::jsbuild`

## How it works

1. `read_project` in [[Python Module (testmap)]] and `JsRepo::config_for` in [[JsRepo]], and 2 more, call `ancestors`.
2. `JsRepo::config_for` and `JsRepo::read` in [[JsRepo]], and 3 more, call `parent_dir`.
3. `JsRepo::manager_of` and `JsRepo::resolve_member` in [[JsRepo]], and 2 more, call `join`.
4. `resolve_ts_import` in [[Lang Module]] and `JsRepo::resolve_import` in [[JsRepo]], and 1 more, call `resolve_source`.
5. `discover` in [[Units Module]] and `JsRepo::resolve_member` in [[JsRepo]] call `resolve_js_entry`. `resolve_js_entry` calls `join`.

8 more entry points: `join_norm`, `match_pattern`, `read_tsconfig`, `export_targets`, `parse_pnpm_workspace`, `glob_matches`, and 2 more.

## Items

- `pub fn parent_dir(path: &str) -> &str` — public function, returns `&str`; called by `JsRepo::config_for`, `JsRepo::read`, `PythonRoots::in_package`, `PythonRoots::project_of` and 2 more.
- `pub fn join(dir: &str, file: &str) -> String` — public function, returns `String`; called by `JsRepo::manager_of`, `JsRepo::resolve_member`, `PythonRoots::in_package`, `PythonRoots::read` and 2 more.
- `pub fn ancestors(dir: &str) -> Vec<&str>` — `dir` and each of its ancestors, nearest first, ending at the root (`""`).
- `pub fn join_norm(dir: &str, target: &str) -> Option<String>` — `target` taken from `dir`, with `.` and `..` folded; `None` when it climbs out of the repository.
- `pub fn strip_jsonc(text: &str) -> String` — `text` without comments and trailing commas, so that a JSON parser reads it. Strings are left alone.
- `pub fn parse_jsonc(text: &str) -> Option<Value>` — A JSON document that may hold comments and trailing commas.
- `pub struct PathRule` — one entry of `compilerOptions.paths`.
- `pub struct TsConfig` — what a `tsconfig.json` or `jsconfig.json` says about how imports resolve.
- `pub struct AliasHit` — what an import specifier names, as far as a config can tell.
- `pub fn read_tsconfig(path: &str, read: &dyn Fn(&str) -> Option<String>) -> Option<TsConfig>` — read the config at `path` and what it extends. A file that is missing or not JSON reads as nothing; an `extends` that cannot be found is skipped.
- `pub fn resolve_source(files: &HashSet<&str>, base: &str) -> Option<String>` — source file `base` names: itself, with an extension, or an `index` file in the folder of that name. `./text.js` is how ESM TypeScript names `./text.ts`.
- `pub fn resolve_js_entry(files: &HashSet<&str>, dir: &str, spec: &str) -> Option<String>` — source file a `package.json` path names. Published packages point at build output, so the matching file under `src` is tried as well.
- [[JsPackage]] — A package of the repository: a `package.json` with a name.
- `fn export_targets(value: &Value, out: &mut Vec<String>)` — the paths an `exports` value points at, the conditions a bundler or a runtime picks first coming first.
- `pub fn parse_pnpm_workspace(text: &str) -> Vec<String>` — the patterns of `pnpm-workspace.yaml`'s `packages:` list. Only the list forms pnpm documents are read: block (`- 'packages/*'`) and flow (`packages: ['a', 'b']`).
- `pub fn glob_matches(pattern: &str, dir: &str) -> bool` — true when the folder `dir` (relative to the workspace root) is named by one glob of a workspace: `*` stands for one folder name, `**` for any number of folders.
- [[Manager]] — enum
- `pub struct PmInfo` — the package manager of one package and where it works from.
- [[JsRepo]] — everything a repository's JavaScript build files say, read once.
- `pub enum JsLock` — the versions a lock file pins. A format the reader does not know is `None` from `JsLock::read`, and so no version is claimed.

And 13 more items.

## Depends on

- [[Manager]] — uses `Manager`

## Used by

- [[JsRepo]] — from `JsRepo`, `config_for`, `find_members`, `manager_of`
- [[Manifests Module]] — from `Packages`, `js_lock`, `js_locked`
- [[Pm Module]] — from `exec`, `install`, `run_script`
- [[PythonRoots]] — from `in_package`, `project_of`, `read`
- [[JsPackage]] — from `entry_specs`, `subpath_specs`
- [[Lang Module]] — from `resolve_ts_import`
- [[Pyroots Module]] — from `root`
- [[Python Module (testmap)]] — from `read_project`

## See also

- [[_index]]
- [[cw-parse]]

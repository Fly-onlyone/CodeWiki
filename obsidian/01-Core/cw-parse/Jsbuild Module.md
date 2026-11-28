---
tags: [core, cw-parse, module]
---

# Jsbuild Module

> What the build files of a JavaScript or TypeScript repository say, read as data and never run: `tsconfig.json` and `jsconfig.json` (`baseUrl`, `paths`, `extends`), `package.json` (name, entry points, workspaces, the package manager), `pnpm-workspace.yaml`, and the lock files of npm, pnpm, Yarn and Bun.

## What it does

Reads the build files of a JavaScript or TypeScript repository as data and never runs them. The files are `tsconfig.json`, `jsconfig.json`, `package.json`, `pnpm-workspace.yaml` and the lock files of npm, pnpm, Yarn and Bun. The scanner needs this to turn an import such as `@/utils/text` or `@acme/ui` into a file of the repository, to find workspace members, to pick the package manager, and to know which version a lock file pins. The YAML readers go line by line and know only the shapes the tests pin. Anything else reads as empty, never as a guess.

## Source

- `crates/cw-parse/src/jsbuild.rs` — module `cw_parse::jsbuild`

## How it works

Small path helpers (`parent_dir`, `join`, `ancestors`, `join_norm`) work on repo-relative strings. `join_norm` folds `.` and `..` and gives `None` when a path climbs out of the repository.

`strip_jsonc` removes comments and trailing commas but leaves strings alone. `parse_jsonc` also drops a byte-order mark.

`read_tsconfig` loads a config and follows `extends` up to 8 levels deep. A parent can be a relative file or a package in `node_modules` up the tree. Later parents win over earlier ones, and the file wins over all. `TsConfig::lookup` picks the `paths` pattern with the longest prefix before its `*`, then adds `baseUrl` itself.

`resolve_source` finds a file for a base path: the file itself, with each TS or JS extension, or an `index` file in a folder. `resolve_js_entry` does this for a `package.json` entry and also tries `src` when the entry points into `dist`, `build`, `lib` or `out`.

`PmInfo` and `Manager` describe the package manager. `PmInfo::install` gives the strict command for a lock file, such as `npm ci`. [[JsRepo]] ties it together. `JsLock::read` picks a reader by file name and `JsLock::version` finds the pinned version of a package.

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

## Gotchas

- Bundler aliases in `vite.config.ts` or webpack are code and are not read.
- `paths` of a file replace inherited ones, as TypeScript does. Without `baseUrl`, targets are relative to the config that wrote `paths`.
- A lock file of an unknown format version gives `None`, so no version is claimed. Known: npm 1 to 3, pnpm 6.0 and 9.0, Yarn Berry metadata 4 to 9, bun 0 and 1.
- `parse_pnpm_workspace` reads only the block and flow list forms of `packages:`.
- For Yarn, a requirement missing from the lock falls back to the only version present, and only when exactly one exists.

## See also

- [[_index]]
- [[cw-parse]]

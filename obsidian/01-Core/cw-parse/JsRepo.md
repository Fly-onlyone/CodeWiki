---
tags: [core, cw-parse]
---

# JsRepo

> Everything a repository's JavaScript build files say, read once.

## What it does

`JsRepo` reads every JavaScript build file of a repository once and then answers questions. Which packages exist? Which belong to a workspace? Which package manager does a folder use? Which file does an import such as `@/store/cart` or `@acme/ui` name? Nothing is run: files are read as data through a `read` callback the caller supplies.

## Source

- `crates/cw-parse/src/jsbuild.rs` — definition of `JsRepo`

## How it works

`JsRepo::read` loops over the file paths and picks files by name. A `package.json` becomes a [[JsPackage]], and its `workspaces` globs are noted (an array, or an object with `packages`). A `pnpm-workspace.yaml` adds globs through `parse_pnpm_workspace`. A `tsconfig.json` is read with `read_tsconfig`. A `jsconfig.json` is used only where its folder has no tsconfig. Then it notes each `yarn.lock` (Berry or not, by its `__metadata:` header) and each `.yarnrc.yml` folder, and calls `find_members`.

`find_members` tests each package against each workspace root with `glob_matches`. Globs run in order. A `!` glob removes a package that an earlier glob added. The workspace root's own package is not a member.

`manager_of` picks the manager in this order. First the `packageManager` field of the nearest `package.json` that has one. Then the nearest lock file above. Then a `pnpm-workspace.yaml`. Then npm. The install folder is the lock file's folder, else the workspace root, else the package.

`resolve_import` returns a file and an alias flag. It tries `paths` and `baseUrl` of the nearest config first, then `resolve_member` for a workspace package name. Relative specifiers return `(None, false)`.

## Items

Declared as `pub struct JsRepo`.

Members:

- `pub fn read(paths: &[String], read: &dyn Fn(&str) -> Option<String>) -> Self` — read the build files among `paths` through `read`.
- `pub fn package_at(&self, dir: &str) -> Option<&JsPackage>` — public method, returns `Option<&JsPackage>`; called by `JsRepo::manager_of`.
- `pub fn has_workspaces(&self) -> bool` — public method, returns `bool`.
- `pub fn members(&self) -> Vec<(&JsPackage, &str)>` — the packages that a workspace lists, with the root folder of the workspace that lists each. A package of the workspace root itself is not among them.
- `fn find_members(&mut self)` — private method; called by `JsRepo::read`.
- `fn member_at(&self, dir: &str) -> Option<(&JsPackage, String)>` — the workspace member in `dir`, with the root of its workspace.
- `pub fn manager_of(&self, dir: &str) -> PmInfo` — the package manager of the package in `dir`: what the nearest `package.json` that says so names in `packageManager`, else the lock file nearest above, else a workspace file, else npm.
- `pub fn config_for(&self, file: &str) -> Option<&TsConfig>` — the config that applies to a file: the nearest one up the tree.
- `pub fn resolve_member(&self, files: &HashSet<&str>, spec: &str) -> Option<String>` — the file a workspace package's name or subpath names.
- `resolve_import` — the repository file an import of `spec` in `from` names, and whether the specifier is an alias of the repository whether or not a file was found. The order: `paths`, `baseUrl`, a workspace package by name.

## Depends on

- [[Jsbuild Module]] — uses `LOCK_FILES`, `PmInfo`, `TsConfig`, `ancestors`
- [[JsPackage]] — uses `JsPackage`, `entry_specs`, `subpath_specs`
- [[Manager]] — uses `Manager`

## Used by

- [[Testmap Module]] — from `Env`, `gather`
- [[Js Module]] — from `command`, `package_runner`
- [[Scan Module]] — from `scan_repo`
- [[Units Module]] — from `discover`

## Gotchas

- A `packageManager` field wins over a lock file of another manager. A lock file of that manager is then looked for.
- `bun.lockb` is binary. It names bun but counts as no lock file.
- A yarn lock counts as Berry from a `packageManager` major of 2 or more, else from `.yarnrc.yml` or the lock header.
- The alias flag can be true with no file found. It means the import is the repository's own, not an outside package.
- A package with no `name` can never be resolved by name.

## See also

- [[_index]]
- [[Jsbuild Module]]

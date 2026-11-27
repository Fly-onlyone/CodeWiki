---
tags: [core, cw-parse]
---

# JsRepo

> Everything a repository's JavaScript build files say, read once.

## Source

- `crates/cw-parse/src/jsbuild.rs` — definition of `JsRepo`

## How it works

1. `scan_repo` in [[Scan Module]] and `gather` in [[Testmap Module]], and 1 more, call `read`. `read` calls `parent_dir`, then `parse_pnpm_workspace`, then `read_tsconfig` (all in [[Jsbuild Module]]), then `find_members`. `find_members` calls `glob_matches` in [[Jsbuild Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[Jsbuild Module]]

---
tags: [core, cw-parse]
---

# Manager

> `Manager` enum defined in `cw_parse::jsbuild` with 1 methods.

## What it does

`Manager` names the JavaScript package manager of a folder: npm, pnpm, Yarn or Bun. [[JsRepo]] decides which one applies, and other code uses the answer to build install and run commands.

## Source

- `crates/cw-parse/src/jsbuild.rs` — definition of `Manager`

## How it works

`name` returns the command word: `npm`, `pnpm`, `yarn` or `bun`. The value comes from `parse_package_manager` (the `packageManager` field of `package.json`, such as `pnpm@9.1.0`), from the lock file found by the `LOCK_FILES` table, or from a `pnpm-workspace.yaml`. If none says anything, npm is used. The `PmInfo` struct in [[Jsbuild Module]] pairs the manager with its lock file and install folder.

## Items

Declared as `pub enum Manager`.

Members:

- `pub fn name(self) -> &'static str` — public method, returns `&'static str`.

## Used by

- [[Jsbuild Module]] — from `LOCK_FILES`, `PmInfo`, `parse_package_manager`
- [[Pm Module]] — from `in_folder`
- [[JsRepo]] — from `manager_of`

## Gotchas

- Both `bun.lock` and `bun.lockb` map to Bun, but only the text form `bun.lock` is read.
- The order of `LOCK_FILES` decides when a folder holds several lock files: pnpm, then Yarn, then Bun, then npm.

## See also

- [[_index]]
- [[Jsbuild Module]]

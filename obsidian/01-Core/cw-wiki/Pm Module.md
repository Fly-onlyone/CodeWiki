---
tags: [core, cw-wiki, module]
---

# Pm Module

> The commands of a JavaScript or TypeScript package, written for its package manager (`npm`, `pnpm`, `yarn` or `bun`; see `cw_parse::jsbuild` for how the manager is found).

## What it does

Every command runs from the repository root. A package that a workspace lists is named with the manager's own filter, `npm -w <name>`, `pnpm --filter <name>`, `yarn workspace <name>`, `bun run --filter <name>`; any other package below the root is entered with `cd <folder> &&`, except where npm's `--prefix` has always served (the operations note).

## Source

- `crates/cw-wiki/src/pm.rs` — module `cw_wiki::pm`

## How it works

1. `JsTests::command` in [[Js Module]] and `RenderCtx::unit_commands` in [[RenderCtx]] call `run_script`. `run_script` calls `in_folder`. `in_folder` calls `arg`.
2. `JsTests::command` in [[Js Module]] calls `arg`.
3. `RenderCtx::unit_commands` in [[RenderCtx]] calls `install`. `install` calls `in_folder`.
4. `JsTests::command` in [[Js Module]] calls `exec`. `exec` calls `arg`.

## Items

- `pub fn arg(text: &str) -> String` — A name or path as one shell argument: in double quotes when it holds a character a shell reads as syntax (`@scope/name` starts a splat in PowerShell).
- `fn in_folder(manager: Manager, dir: &str, command: &str) -> String` — `command` run in the folder `dir`: npm takes `--prefix`, the others a change of folder.
- `pub fn install(pm: &PmInfo) -> String` — the command that installs what the lock file pins, where the lock file is.
- `pub fn run_script(pm: &PmInfo, dir: &str, script: &str) -> String` — the command that runs the script `script` of the package in `dir`.
- `pub fn exec(pm: &PmInfo, dir: &str, program: &str) -> Option<String>` — the command that runs a program of the package's dependencies (`vitest run a.test.ts`), with paths as the package's own folder sees them. `None` when the manager has no way to do that for a workspace package.

## Depends on

- [[Jsbuild Module]] — uses `PmInfo`, `install`
- [[Manager]] — uses `Manager`

## Used by

- [[Js Module]] — from `command`
- [[RenderCtx]] — from `unit_commands`

## See also

- [[_index]]
- [[cw-wiki]]

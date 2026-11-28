---
tags: [core, cw-wiki, module]
---

# Pm Module

> The commands of a JavaScript or TypeScript package, written for its package manager (`npm`, `pnpm`, `yarn` or `bun`; see `cw_parse::jsbuild` for how the manager is found).

## What it does

Writes the shell commands for a JavaScript or TypeScript package the way its own package manager spells them. The same idea, such as run the `test` script, is `npm -w name run test`, `pnpm --filter name run test`, `yarn workspace name run test` or `bun run --filter name test`. The wiki shows these commands in notes about operations and tests. Every command is meant to run from the repository root. How the manager is found is the job of [[Jsbuild Module]]; this module only writes strings.

## Source

- `crates/cw-wiki/src/pm.rs` — module `cw_wiki::pm`

## How it works

`run_script` checks whether the package is a workspace member (`PmInfo` has a member name). If it is, the manager's filter flag names it. If not, `in_folder` enters the folder: npm gets `--prefix <dir>`, the others get `cd <dir> &&`. At the root nothing is added.

`install` asks `PmInfo::install` for the strict command (for example `npm ci`, with a lock file) and runs it in the folder that holds the lock file.

`exec` writes the command that runs a program of the package's dependencies, such as `vitest run a.test.ts`. A member uses the manager's filter form. A plain package uses `npx`, `pnpm exec`, `yarn` or `bunx`, after a `cd` when needed.

`arg` quotes a name or path when it holds anything but letters, digits and `. _ - /`.

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

## Gotchas

- `exec` gives `None` for a Bun workspace member. Bun has no way to run a program for one member this way.
- `in_folder` uses `--prefix` only for npm commands that do not start with `exec`. For those, `cd` is used.
- `@scope/name` is quoted because PowerShell reads `@` as the start of a splat.

## See also

- [[_index]]
- [[cw-wiki]]

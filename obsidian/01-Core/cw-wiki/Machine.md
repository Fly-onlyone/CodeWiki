---
tags: [core, cw-wiki]
---

# Machine

> Reads the files a build on this machine left behind: the sources cargo unpacked into its registry cache, and `node_modules`.

## What it does

A note about an outside package, such as a crate or an npm package, should say what the package is. That text is in no file of the repository. `Machine` is the `PackageInfo` that finds it in files a build left on this computer. It makes no network call. On a machine that has never built the repository it finds nothing.

## Source

- `crates/cw-wiki/src/manifests.rs` — definition of `Machine`

## How it works

`Machine::new` takes the cargo home from `CARGO_HOME`, or from a `.cargo` folder in the user's home folder. `with_cargo_home` lists the folders under that home in `registry/src`, one per registry, in sorted order.

`describe` depends on the language. For Rust it looks for a folder named after the package and its locked version in each registry, reads its `Cargo.toml` and takes the package description. Without a locked version it takes the newest copy whose folder name starts with the package name and a digit. For TypeScript and JavaScript it reads `node_modules/<package>/package.json` and returns the description with whitespace folded to single spaces. Other languages give nothing.

## Items

Declared as `pub struct Machine`.

Members:

- `pub fn new(root: &Path) -> Self` — for the repository at `root`, with the cargo home the environment names (`CARGO_HOME`, or `.cargo` in the user's home folder).
- `pub fn with_cargo_home(root: &Path, cargo_home: Option<&Path>) -> Self` — as `Machine::new`, with the cargo home given.
- `fn describe(&self, language: Language, package: &str, version: Option<&str>) -> Option<String>` — public method, returns `Option<String>`.

And 1 more private item with no comment.

## Depends on

- [[Workspace Module]] — uses `package_description`
- [[Manifests Module]] — uses `PackageInfo`
- [[Language]] — uses `Language`

## Used by

- [[Service Module]] — from `run_update`

## Gotchas

- Results differ between machines. That is why [[Manifests Module]] saves what was found in `packages.json` and falls back to it.
- Without a lock file the newest cached copy may not be the version the repository uses.
- A missing registry folder or an unreadable file gives no description, with no error.
- `node_modules` is read relative to the repository root only, not from parent folders.

## See also

- [[_index]]
- [[Manifests Module]]

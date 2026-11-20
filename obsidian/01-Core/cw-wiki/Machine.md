---
tags: [core, cw-wiki]
---

# Machine

> Reads the files a build on this machine left behind: the sources cargo unpacked into its registry cache, and `node_modules`.

## What it does

It never asks the network, and it knows nothing on a machine that has not built the repository.

## Source

- `crates/cw-wiki/src/manifests.rs` — definition of `Machine`

## How it works

1. `run_update` in [[Service Module]] calls `new`. `new` calls `with_cargo_home`.

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

## See also

- [[_index]]
- [[Manifests Module]]

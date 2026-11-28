---
tags: [core, cw-wiki]
---

# Machine

> Reads the files a build on this machine left behind: the sources cargo unpacked into its registry cache, and `node_modules`.

## What it does

A note about an outside package, such as a crate, an npm package or a NuGet package, should say what the package is. That text is in no file of the repository. `Machine` is the `PackageInfo` that finds it in files a build left on this computer. It makes no network call. On a machine that has never built the repository it finds nothing.

## Source

- `crates/cw-wiki/src/manifests.rs` — definition of `Machine`

## How it works

`Machine::new` takes the cargo home from `CARGO_HOME`, or from a `.cargo` folder in the user's home folder, and the NuGet folder from `NUGET_PACKAGES`, or `.nuget/packages` in the home folder. `with_cargo_home` lists the folders under `registry/src`, one per registry, in sorted order. `with_nuget` keeps the NuGet folder only if it exists.

`describe` depends on the language:

- Rust: it looks in each registry for a folder named after the package and its locked version, reads its `Cargo.toml` and takes the package description. Without a locked version it takes the newest copy whose folder name starts with the package name and a digit.
- TypeScript and JavaScript: it reads `package.json` under `node_modules/<package>`, with whitespace folded to single spaces. `describe_in` also looks in the `node_modules` of each workspace folder first, then at the repository root, then in pnpm's hoisted store.
- C#: it finds the NuGet folder named like the package, else the first that continues it, takes the locked version folder or the newest, and reads `description` from the `.nuspec`.

Other languages give nothing.

## Items

Declared as `pub struct Machine`.

Members:

- `pub fn new(root: &Path) -> Self` — for the repository at `root`, with the cargo home the environment names (`CARGO_HOME`, or `.cargo` in the user's home folder).
- `pub fn with_nuget(mut self, folder: Option<PathBuf>) -> Self` — the same machine with NuGet's package folder given.
- `pub fn with_cargo_home(root: &Path, cargo_home: Option<&Path>) -> Self` — as `Machine::new`, with the cargo home given.
- `fn nuget_description(&self, package: &str, version: Option<&str>) -> Option<String>` — the `description` of the NuGet package a namespace root names, from the `.nuspec` NuGet unpacked next to the package. NuGet names its folders by the lowercase id, so the package is the folder named like `package`, else the first one that continues it.
- `fn node_description(&self, package: &str, dirs: &[&str]) -> Option<String>` — the `description` of the `package.json` that `node_modules` holds for `package`: in the folders `dirs` (each package of a workspace has its own), then at the repository root, then in pnpm's hoisted store. A pnpm `node_modules/<name>` is a link into `.pnpm`, and reading through it needs nothing more than reading a folder.
- `describe_in` — public method, returns `Option<String>`.
- `fn describe(&self, language: Language, package: &str, version: Option<&str>) -> Option<String>` — public method, returns `Option<String>`; called by `Machine::describe_in`.

And 1 more private item with no comment.

## Depends on

- [[Manifests Module]] — uses `PackageInfo`, `nuget`
- [[Workspace Module]] — uses `package_description`
- [[Units Module]] — uses `xml_element`
- [[Language]] — uses `Language`

## Used by

- [[cw-wiki]] — from `the_installed_description_of_a_package_reads_through_node_modules`
- [[Service Module]] — from `run_update`

## Gotchas

- Results differ between machines. That is why [[Manifests Module]] saves what was found in `packages.json` and falls back to it.
- Without a lock file the newest cached copy may not be the version the repository uses.
- A missing registry folder or an unreadable file gives no description, with no error.
- NuGet names its folders in lowercase, so the lookup lowercases the package name.
- Besides the repository root, `node_modules` is read only from the workspace folders it is given, not from parent folders.

## See also

- [[_index]]
- [[Manifests Module]]

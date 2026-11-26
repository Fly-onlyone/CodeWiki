---
tags: [core, cw-wiki, module]
---

# Manifests Module

> What a repository's build files say about a package from outside it: the version asked for, the version locked, the features turned on, and what the package is.

## What it does

Notes about outside packages need facts: the version asked for, the version locked, the features turned on, and what the package is. This module reads the first three from the build files of the repository and keeps the fourth in a saved file, so that notes come out the same on every machine.

## Source

- `crates/cw-wiki/src/manifests.rs` — module `cw_wiki::manifests`

## How it works

`Packages::read` reads the root `Cargo.toml` and `Cargo.lock` of the repository. The lock gives exact versions by crate name, and the published name (for example `tree-sitter` for `tree_sitter`).

`declared` answers what each build file asks of a package. For Rust units, `cargo` reads the dependency spec: requirement, features, and whether default features are off. A `workspace = true` spec is resolved through the root manifest. `Packages::locked` picks the locked version that the requirement accepts, using Cargo's default rule in `accepts`. If that cannot be told and the lock file has only one version, that one is used. For other units, `other` reads the requirement by file kind. Maven group matching, gradle strings and Python specifiers each have a helper.

`describe` adds a one-line description for each package. It asks a `PackageInfo` (see [[Machine]]) with the published name and the locked version. If that finds nothing, it takes the text saved by an earlier run in `packages.json` under the vault's manifest folder. It rewrites the file only when the result differs. An empty result deletes the file.

## Items

- `const SAVED: &str` — file under the vault's manifest folder that keeps the descriptions.
- `pub struct Declared` — what one build file asks of a package.
- `pub struct Packages` — what the repository's own files say about outside packages, beyond each unit's build file, and what is known of each package.
- `fn numbers(text: &str) -> Option<Vec<u64>>` — the leading numbers of a version or of a plain requirement: `1.0.229`, `0.8`. `None` for anything else, such as `>=1, <3` or `*`.
- `fn accepts(asked: &[u64], version: &[u64]) -> bool` — Cargo's default rule: `version` is at least `asked` and equal to it up to the first number that is not zero.
- `pub fn declared(repo: &Repo, package: &str) -> Vec<Declared>` — what each build file that names `package` asks of it, one entry for each file. `package` is the name integrations go by: a crate as used in paths, an npm package, the first segment of a Python import, the first two of a Java one.
- `fn cargo(unit: &Unit, package: &str, packages: &Packages) -> Option<Declared>` — private function, returns `Option<Declared>`; called by `declared`.
- `fn other(unit: &Unit, package: &str) -> Vec<Declared>` — private function, returns `Vec<Declared>`; called by `declared`.
- `fn asked_in(manifest: &Manifest, package: &str) -> Option<(String, Vec<String>)>` — the requirement and the extras `manifest` gives for `package`, read by the rules of the file's kind.
- `fn in_group(group: &str, package: &str) -> bool` — true when the Maven group `group` is the one a Java import of `package` (its first two segments) comes from.
- `fn element<'a>(xml: &'a str, tag: &str) -> Option<&'a str>` — text of the first `<tag>` element in `xml`.
- `fn gradle(text: &str, package: &str) -> Option<String>` — gradle names a dependency in a string, `"group:artifact:version"`.
- `fn python(text: &str, package: &str) -> Option<(String, Vec<String>)>` — Python build files are of many kinds (`requirements.txt`, `pyproject.toml`, `setup.py`, `Pipfile`), so the text after the name is read as it stands: extras in brackets, then the version specifier.
- `pub trait PackageInfo` — says what a package from outside the repository is.
- `pub struct NoPackageInfo` — knows no package. A run with it writes what `packages.json` holds.
- [[Machine]] — reads the files a build on this machine left behind: the sources cargo unpacked into its registry cache, and `node_modules`.
- `struct Saved` — one package in `packages.json`.
- `fn saved_path(vault: &Path) -> PathBuf` — private function, returns `PathBuf`; called by `describe`, `load`.
- `fn load(vault: &Path) -> BTreeMap<String, String>` — private function, returns `BTreeMap<String, String>`; called by `describe`.
- `describe` — say what each of `packages` is and keep the answers in the vault at `vault`. What `info` finds on this machine wins; what it does not find is taken from the file an earlier run wrote. The file then holds exactly the packages named here that have a description, and is written only when that differs from what it held.

And 2 more private items with no comment.

## Depends on

- [[Workspace Module]] — uses `Locked`, `dependency_specs`, `locked_versions`
- [[Unit]] — uses `Unit`, `declares`, `is_rust`
- [[Error]] — uses `external`, `io`
- [[Language]] — uses `Language`, `from_path`
- [[cw-core]] — uses `MANIFEST_DIR`
- [[Error Module]] — uses `Result`
- [[Units Module]] — uses `Manifest`
- [[Repo]] — uses `Repo`

## Used by

- [[Repo]] — from `Repo`, `at`, `build`
- [[Generate Module]] — from `generate`, `generate_with`
- [[cw-wiki]] — from `KnowsSerde`
- [[Machine]] — from `Machine`
- [[RenderCtx]] — from `render_integration`

## Gotchas

- Requirement and locked version come from repository files, so every machine reads the same. Only the description depends on the machine.
- `numbers` accepts only plain versions of one to three numbers. A range like `>=1, <3` or `*` gives no match, so the locked version is chosen only when there is exactly one.
- A package that is no longer named in this run is dropped from `packages.json`.
- `NoPackageInfo` knows nothing, so a run with it just replays the saved file.

## See also

- [[_index]]
- [[cw-wiki]]

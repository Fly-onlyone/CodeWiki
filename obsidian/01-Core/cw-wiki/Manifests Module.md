---
tags: [core, cw-wiki, module]
---

# Manifests Module

> What a repository's build files say about a package from outside it: the version asked for, the version locked, the features turned on, and what the package is.

## What it does

Notes about outside packages need facts: the version asked for, the version locked, the features turned on, and what the package is. This module reads the first three from the build files of the repository, for Rust, JavaScript, Python, Java and Kotlin, Go, C#, and C and C++. It keeps the fourth in a saved file, so that notes come out the same on every machine.

## Source

- `crates/cw-wiki/src/manifests.rs` — module `cw_wiki::manifests`

## How it works

`Packages::read` reads the root `Cargo.toml` and `Cargo.lock`. The lock gives exact versions by crate name, and the published name (for example `tree-sitter` for `tree_sitter`).

`declared` answers what each build file asks of a package, one entry per file. The reader depends on the unit's language:

- Rust: `cargo` reads the dependency spec: requirement, features, and whether default features are off. A `workspace = true` spec is resolved through the root manifest. `Packages::locked` picks the locked version that the requirement accepts, by Cargo's default rule in `accepts`; if that cannot be told and the lock has only one version, that one is used.
- C#: `nuget` reads project files, central versions and the lock file. Go: `go` reads `go.mod` and `go.sum`. C and C++: `cfamily` reads CMake, vcpkg and Conan files.
- JavaScript: the requirement comes from `package.json`, the locked version from the unit's lock file via `js_locked`.
- Java, Kotlin and Python: `other` matches Maven groups, Gradle strings and version catalogs, and Python specifiers.

`describe` adds a one-line description for each package. It asks a `PackageInfo` (see [[Machine]]) with the published name and the locked version. If that finds nothing, it takes the text saved by an earlier run in `packages.json`. It rewrites the file only when the result differs. An empty result deletes the file.

## Items

- `const SAVED: &str` — file under the vault's manifest folder that keeps the descriptions.
- `pub struct Declared` — what one build file asks of a package.
- `pub struct Packages` — what the repository's own files say about outside packages, beyond each unit's build file, and what is known of each package.
- `pub fn declared(repo: &Repo, package: &str) -> Vec<Declared>` — what each build file that names `package` asks of it, one entry for each file. `package` is the name integrations go by: a crate as used in paths, an npm package, the first segment of a Python import, the first two of a Java one.
- `fn cargo(unit: &Unit, package: &str, packages: &Packages) -> Option<Declared>` — private function, returns `Option<Declared>`; called by `declared`.
- `fn other(unit: &Unit, package: &str) -> Vec<Declared>` — private function, returns `Vec<Declared>`; called by `declared`.
- `js_locked` — the version a JavaScript lock file pins for `package`, which the `package.json` `manifest` asks for as `requirement`; empty when the unit has no lock file the reader knows.
- `fn cfamily(unit: &Unit, package: &str) -> Vec<Declared>` — what the build files of a C or C++ unit ask of a library: CMake's `find_package` and `FetchContent_Declare`, vcpkg's `dependencies` (with its `overrides` as the version in effect and its `builtin-baseline` where no version is written), Conan's `requires`, and Conan's lock file for the version in effect. One entry for each file that names the library.
- `fn jvm_requirement(unit: &Unit, manifest: &Manifest, package: &str) -> Option<String>` — what a Gradle or Maven build file asks of `package` when its text does not name the package itself: by the library's artifact, or by its alias in the version catalog the unit keeps.
- `fn asked_in(manifest: &Manifest, package: &str) -> Option<(String, Vec<String>)>` — the requirement and the extras `manifest` gives for `package`, read by the rules of the file's kind.
- `fn go(unit: &Unit, package: &str) -> Vec<Declared>` — what a Go module's `go.mod` asks of the module `package` (an import path outside the repository, cut to the module that holds it): the version of its `require` line, marked when the line is `// indirect` or a `replace` sends the module elsewhere, and the version `go.sum` holds a hash of.
- `fn nuget(unit: &Unit, package: &str) -> Vec<Declared>` — what the project files of a C# unit ask of `package`: the version written beside the reference or, for a reference with none, the central one, and the version the lock file pins.
- `fn in_group(group: &str, package: &str) -> bool` — true when the Maven group `group` is the one a Java import of `package` (its first two segments) comes from.
- `fn names_package(group: &str, artifact: &str, package: &str) -> bool` — true when the library `group:artifact` is the one a Java or Kotlin import of `package` comes from: its group is the package or starts with it, or, for the `kotlinx` libraries, whose group (`org.jetbrains.kotlinx`) does not say, its artifact (`kotlinx-coroutines-core`) starts with the package's name (`kotlinx.coroutines`).
- `pub trait PackageInfo` — says what a package from outside the repository is.
- `pub struct NoPackageInfo` — knows no package. A run with it writes what `packages.json` holds.
- [[Machine]] — reads the files a build on this machine left behind: the sources cargo unpacked into its registry cache, and `node_modules`.
- `fn saved_path(vault: &Path) -> PathBuf` — private function, returns `PathBuf`; called by `describe`, `load`.
- `fn load(vault: &Path) -> BTreeMap<String, String>` — private function, returns `BTreeMap<String, String>`; called by `describe`.
- `describe` — say what each of `packages` is and keep the answers in the vault at `vault`. What `info` finds on this machine wins; what it does not find is taken from the file an earlier run wrote. The file then holds exactly the packages named here that have a description, and is written only when that differs from what it held.

And 14 more items.

## Depends on

- [[Cdeps Module]] — uses `names_library`, `parse_conan_lock`, `parse_conanfile_py`, `parse_conanfile_txt`
- [[Units Module]] — uses `Manifest`, `xml_attribute`, `xml_element`, `xml_elements`
- [[Jsbuild Module]] — uses `JsLock`, `read`, `version`
- [[Workspace Module]] — uses `Locked`, `dependency_specs`, `locked_versions`
- [[Language]] — uses `Language`, `from_path`, `is_c_family`
- [[Unit]] — uses `Unit`, `declares`, `is_rust`
- [[Gomod Module]] — uses `go_sum_versions`, `parse_go_mod`
- [[Gradle Module]] — uses `accessor`, `parse_version_catalog`

## Used by

- [[Repo]] — from `Repo`, `at`, `build`
- [[Generate Module]] — from `generate`, `generate_with`
- [[Machine]] — from `Machine`, `new`
- [[cw-wiki]] — from `KnowsSerde`
- [[RenderCtx]] — from `render_integration`

## Gotchas

- Requirement and locked version come from repository files, so every machine reads the same. Only the description depends on the machine.
- `numbers` accepts only plain versions of one to three numbers. A range like `>=1, <3` or `*` gives no match, so the locked version is chosen only when there is exactly one.
- A package that is no longer named in this run is dropped from `packages.json`.
- `NoPackageInfo` knows nothing, so a run with it just replays the saved file.
- A Rust manifest that was not read still counts: the names the scan found under dependencies give a bare entry without versions.
- A JavaScript lock file in a format the reader does not know gives no locked version, and is not parsed again.

## See also

- [[_index]]
- [[cw-wiki]]

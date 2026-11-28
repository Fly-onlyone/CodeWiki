---
tags: [core, cw-parse]
---

# JsPackage

> A package of the repository: a `package.json` with a name.

## What it does

Holds one `package.json` of the repository that has a name: its folder, its name and its raw JSON. [[JsRepo]] builds one for every `package.json` it can read. The scanner asks a package for its dependencies and for the files a bare import of it may mean.

## Source

- `crates/cw-parse/src/jsbuild.rs` — definition of `JsPackage`

## How it works

`dependencies` reads four tables in order: `dependencies`, `devDependencies`, `peerDependencies`, `optionalDependencies`. A spec that is not a string becomes an empty string.

`entry_specs` lists the files an import of the package name may mean, best first. First the targets of `exports` for `.`, found by `export_targets`. If `exports` has no key starting with a dot, the whole value counts as the root. Then come `source`, `module`, `main`, `types` and `typings`.

`subpath_specs` does the same for `./sub`. It matches every `exports` key with `match_pattern`, and a `*` in the key is replaced in the target by what the pattern captured. The results are paths as written in the file, often into build output. `resolve_js_entry` in [[Jsbuild Module]] later maps them to source files.

## Items

Declared as `pub struct JsPackage`.

Members:

- `pub fn dependencies(&self) -> Vec<(String, String)>` — every table of `package.json` that names the packages it needs.
- `pub fn json(&self) -> &Value` — public method, returns `&Value`.
- `fn entry_specs(&self) -> Vec<String>` — files a bare import of the package may mean, best first: the targets of `exports` for `.`, then `source`, `module`, `main`, `types`.
- `fn subpath_specs(&self, sub: &str) -> Vec<String>` — targets of `exports` for the subpath `./sub`, patterns with `*` included.

## Depends on

- [[Jsbuild Module]] — uses `export_targets`, `match_pattern`

## Used by

- [[JsRepo]] — from `JsRepo`, `member_at`, `members`, `package_at`
- [[Js Module]] — from `command`, `package_runner`
- [[Units Module]] — from `discover`

## Gotchas

- `entry_specs` and `subpath_specs` are private. Callers go through `JsRepo::resolve_member`.
- Only `exports` of the object form is read for subpaths. A string `exports` gives none.
- The `package.json` is kept whole in a private field. Use `json` to read it.

## See also

- [[_index]]
- [[Jsbuild Module]]

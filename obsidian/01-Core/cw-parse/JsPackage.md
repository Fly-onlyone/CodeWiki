---
tags: [core, cw-parse]
---

# JsPackage

> A package of the repository: a `package.json` with a name.

## Source

- `crates/cw-parse/src/jsbuild.rs` — definition of `JsPackage`

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

## See also

- [[_index]]
- [[Jsbuild Module]]

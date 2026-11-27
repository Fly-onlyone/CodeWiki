---
tags: [core, cw-parse, module]
---

# Gomod Module

> What Go's build files say: `go.mod`, `go.work` and `go.sum`, read as text.

## What it does

The formats are documented at <https://go.dev/ref/modgo-mod-file> and <https://go.dev/ref/modworkspaces>. A `go.mod` is a list of directives, one per line or a parenthesised block of lines, with `//` comments; the scanner needs only `module`, `go`, `require` and `replace`.

## Source

- `crates/cw-parse/src/gomod.rs` — module `cw_parse::gomod`

## How it works

1. `scan_repo` in [[Scan Module]] and `go` in [[Manifests Module]], and 1 more, call `parse_go_mod`. `parse_go_mod` calls `code`, then `directive`, then `unquote`. `directive` calls `code`.
2. `discover` in [[Units Module]] calls `parse_go_work`. `parse_go_work` calls `code`, then `unquote`, then `directive`.
3. `go` in [[Manifests Module]] calls `go_sum_versions`. `go_sum_versions` calls `version_key`.
4. `Extractor::import` in [[Go Module]] calls `guess_package_name`. `guess_package_name` calls `sanitize` in [[Lang Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub struct GoRequire` — one `require` line.
- `pub struct GoReplace` — one `replace` line: `old [version] => new [version]`.
- `pub struct GoMod` — what a `go.mod` file says.
- `fn code(line: &str) -> &str` — text before a `//` comment, trimmed. A `//` inside a quoted path is not a comment, and no module path holds one.
- `fn unquote(text: &str) -> &str` — A path as `go.mod` writes it: plain, or in `"` or backquotes.
- `fn directive(line: &str) -> Option<(&str, &str)>` — the directive of a line and what follows it, `require (` giving a block.
- `pub fn parse_go_mod(text: &str) -> GoMod` — read a `go.mod`. A line it cannot read is left out.
- `pub fn parse_go_work(text: &str) -> Vec<String>` — the directories a `go.work` file lists with `use`, as written.
- `pub fn go_sum_versions(text: &str) -> BTreeMap<String, Vec<String>>` — the versions `go.sum` holds a source hash for, by module path, oldest first. The `/go.mod` lines hash only the module file and are left out.
- `fn version_key(version: &str) -> Vec<u64>` — A sort key that puts `v1.9.0` before `v1.10.0`.
- `pub fn guess_package_name(path: &str) -> String` — the name Go code gives a package imported by `path` when the package's own name is not known: the last segment, without a major-version element (`/v2`), a `.v3` suffix or Go's `go-` and `-go` decoration, with every other non-identifier character turned into `_`.
- [[GoModules]] — the Go modules of a repository: the folder of each `go.mod` and the module path it declares.

## Depends on

- [[Lang Module]] — uses `sanitize`

## Used by

- [[Go Module]] — from `import`
- [[Scan Module]] — from `scan_repo`
- [[Manifests Module]] — from `go`
- [[Units Module]] — from `discover`

## See also

- [[_index]]
- [[cw-parse]]

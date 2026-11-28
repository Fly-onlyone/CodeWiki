---
tags: [core, cw-parse, module]
---

# Gomod Module

> What Go's build files say: `go.mod`, `go.work` and `go.sum`, read as text.

## What it does

Reads Go's build files as plain text, with no Go tool involved. From `go.mod` it takes the module path, the `go` version, the `require` lines and the `replace` lines. From `go.work` it takes the directories listed with `use`. From `go.sum` it takes the versions that have a source hash. The scanner uses this to know which Go module a file belongs to, which dependencies a repository has, and which package name an import path stands for. `GoModules` maps files to packages and import paths to folders.

## Source

- `crates/cw-parse/src/gomod.rs` — module `cw_parse::gomod`

## How it works

`parse_go_mod` walks the file line by line. `code` cuts off a `//` comment. `directive` splits a line into its keyword and the rest. A line such as `require (` opens a block, and every following line counts as that directive until a lone `)`. Only `module`, `go`, `require` and `replace` are kept; other directives such as `exclude` are skipped. A line that ends in `// indirect` marks the require as indirect.

`parse_go_work` works the same way but keeps only `use` paths, from a single line or a block.

`go_sum_versions` groups versions by module path and drops the `/go.mod` lines, which hash only the module file. It then sorts each list with `version_key`, which compares numbers, so v1.9.0 comes before v1.10.0.

`guess_package_name` turns an import path into the name Go code would use: it drops a `/v2` element, a `.v3` suffix and a `go-` prefix or `-go` suffix, then calls `sanitize` from [[Lang Module]].

`GoModules::of` picks the module whose folder is the nearest above a file. `package_of` builds the qualified name from the module path plus the folders below it. `folder_of` goes the other way: the longest matching module path wins.

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

## Gotchas

- A line the parser cannot read is dropped without a message.
- `code` splits on the first `//` anywhere, so a quoted path holding `//` would be cut. The source says no module path holds one.
- `replace` keeps the new version empty when the target is a local directory.
- `folder_of` needs a full path segment match: `example.com/application` does not match module `example.com/app`.
- `version_key` reads a part that is not a number as 0, and ignores pre-release text after `-` or `+`, so those sort as ties.

## See also

- [[_index]]
- [[cw-parse]]

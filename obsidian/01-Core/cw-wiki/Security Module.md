---
tags: [core, cw-wiki, module]
---

# Security Module

> Security pattern findings for Rust source.

## What it does

Each rule is a narrow lexical pattern over code with comments and string contents masked out (`crate::rustscan`), so a pattern never fires on prose or on text inside a literal. Test code is skipped: files under `tests/`, `benches/` and `examples/`, and items behind `#[cfg(test)]` or `#[test]`. The rules favour silence over recall; they flag what is worth a reviewer's look, not everything that could go wrong.

An `unsafe` block is reported only when no `// SAFETY:` comment says why it is sound: in the comments directly above the statement that holds it, or on the first line inside it. Clippy's `undocumented_unsafe_blocks` asks the same of this workspace.

A finding is silenced by a comment on its line or the line above: `// codewiki-allow: security/<rule>`, or `// codewiki-allow` for any rule.

## Source

- `crates/cw-wiki/src/security.rs` — module `cw_wiki::security`

## How it works

1. `run` in [[codewiki]] and `security` in [[Api Module]], and 1 more, call `scan`. `scan` calls `is_test_path` in [[Rustscan Module]], then `scan_source`. `scan_source` calls `is_test_path` in [[Rustscan Module]], then `Scanned::new` in [[Scanned]], then `unsafe_blocks`, then `hardcoded_secrets`, then `command_injection`, then `sql_format`, and 6 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const UNSAFE_BLOCK: &str` — an `unsafe { … }` block with no `// SAFETY:` comment. `unsafe fn`, `impl` and `trait` are declarations and are not reported.
- `pub const HARDCODED_SECRET: &str` — A recognisable credential, or a high-entropy literal assigned to a name such as `token`, `secret`, `password` or `api_key`.
- `pub const COMMAND_INJECTION: &str` — `Command::new` on a shell with a `format!`-built command line, or with a `format!`-built program name.
- `pub const SQL_FORMAT: &str` — A SQL statement passed through `format!` with interpolated values.
- `pub const TLS_DISABLED: &str` — certificate or hostname verification switched off on a TLS client.
- `pub const ENV_SET_VAR: &str` — `std::env::set_var` or `remove_var`.
- `pub const WORLD_WRITABLE: &str` — A Unix file mode whose "other" digit includes write.
- `pub const INSECURE_HTTP: &str` — an `http://` URL literal whose host is not local, private or reserved.
- `pub struct SecurityFinding` — public struct; used by `scan`, `scan_source`, `security`.
- `struct Hit` — A finding before its offset is turned into a line.
- `fn call_args(s: &Scanned, from: usize) -> Option<(usize, &str)>` — text inside the bracket that opens at or after `from`, when only whitespace separates the two.
- `fn unsafe_blocks(s: &Scanned, out: &mut Vec<Hit>)` — private function; called by `scan_source`.
- `fn hardcoded_secrets(s: &Scanned, out: &mut Vec<Hit>)` — private function; called by `scan_source`.
- `fn command_injection(s: &Scanned, out: &mut Vec<Hit>)` — private function; called by `scan_source`.
- `fn is_constant_name(name: &str) -> bool` — A constant by naming convention: `MAX_ROWS`, `COLUMNS`.
- `fn sql_format(s: &Scanned, out: &mut Vec<Hit>)` — private function; called by `scan_source`.
- `const IDENTIFIER_HOSTS: [&str; 12]` — hosts that appear in `http://` identifiers nobody connects to: XML namespaces, schema ids and license URLs.
- `fn has_safety_comment(lines: &[&str], at: usize) -> bool` — true when a `// SAFETY:` comment states why the `unsafe` block on line `at` (1-based) is sound. The comment is in the run of comments directly above the statement that holds the block, found by walking up past the statement's own earlier lines (`let previous =`), or it is the first line inside a block that opens at the end of line `at`.
- `pub fn scan_source(path: &str, source: &str) -> Vec<SecurityFinding>` — findings in one Rust source file. `path` is repo-relative.
- `pub fn scan(root: &Path, index: &Index) -> Result<Vec<SecurityFinding>>` — findings across every indexed Rust file, by file, line and rule.

And 23 more items.

## Depends on

- [[Scanned]] — uses `Scanned`, `in_test`, `line_of`, `literal_at`
- [[Index]] — uses `Index`, `files`
- [[Error Module]] — uses `Result`
- [[Rustscan Module]] — uses `is_test_path`
- [[Level]] — uses `Level`
- [[Literal]] — uses `Literal`

## Used by

- [[Ci Module]] — from `RULES`, `check`
- [[codewiki]] — from `run`
- [[cw-wiki]] — from `scans_source_files_and_skips_tests`
- [[Api Module]] — from `security`

## See also

- [[_index]]
- [[cw-wiki]]

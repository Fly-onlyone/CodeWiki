---
tags: [core, cw-wiki, module]
---

# Security Module

> Security pattern findings for Rust source.

## What it does

Finds common security mistakes in Rust source with a set of narrow text rules. It needs no parser and no network. Each rule looks at code in which comments and string contents are blanked (see [[Rustscan Module]]), so a rule cannot fire on prose or on text inside a string. Test code is skipped. The rules prefer silence over recall: they flag what deserves a reviewer's look. The CLI, the API and the CI check use the findings.

## Source

- `crates/cw-wiki/src/security.rs` — module `cw_wiki::security`

## How it works

`scan` goes over every indexed Rust file outside test folders, reads it from disk and calls `scan_source`. A file deleted since the last index is skipped. `scan_source` builds a [[Scanned]] value and runs eight rules. Each rule adds a `Hit` with an offset, a level and a message:

- unsafe blocks (warning)
- hard-coded secrets (error): known credential shapes such as private keys, AWS key ids, GitHub and Slack tokens; or a high-entropy string of 16 or more characters assigned to a name like `token` or `password`
- command injection (warning): a shell started with a `format!`-built line, or a `format!`-built program name
- SQL built with `format!` and run-time values (warning)
- TLS verification switched off (error)
- `env::set_var` and `remove_var` (warning)
- world-writable file modes (warning)
- `http://` URLs to remote hosts (warning)

Hits inside test regions are dropped. Offsets become line numbers. Unsafe hits are dropped when `has_safety_comment` finds a `// SAFETY:` comment above the statement or on the first line inside the block. A finding is also dropped when the line or the line above holds a `codewiki-allow` comment. Results are sorted by line and rule, with one finding per line and rule.

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

## Gotchas

- To silence one rule write `// codewiki-allow: security/<rule>` on the line or the line above. A bare `// codewiki-allow` silences all rules there.
- The SAFETY comment is searched upward past the statement's own earlier lines, such as a `let x =` line, and past attributes. A blank line or code between comment and statement breaks the link.
- Secret detection ignores names ending in `_env`, `_path`, `_url`, `_id` and similar. It also ignores placeholder words like `example`, ALL_CAPS names, URLs and values without both letters and digits.
- Plain HTTP is only flagged for remote hosts. Localhost, private ranges, reserved top-level names, single-label names and schema or XML namespace hosts are exempt.
- Only Rust files are scanned.

## See also

- [[_index]]
- [[cw-wiki]]

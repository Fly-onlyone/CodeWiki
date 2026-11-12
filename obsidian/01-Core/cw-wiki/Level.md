---
tags: [core, cw-wiki]
---

# Level

> `Level` enum defined in `cw_wiki::level` with 2 methods.

## Source

- `crates/cw-wiki/src/level.rs` — definition of `Level`

## How it works

Declared as `pub enum Level`.

Public API:

- `pub fn parse(text: &str) -> Option<Self>`
- `pub fn as_str(self) -> &'static str`

## Used by

- [[Ci Module]] — from `CheckFinding`, `fails`, `to_junit`
- [[Security Module]] — from `Hit`, `SecurityFinding`
- [[codewiki]] — from `run`
- [[cw-wiki]] — from `scans_source_files_and_skips_tests`
- [[Api Module]] — from `security`

## See also

- [[_index]]
- [[Level Module]]

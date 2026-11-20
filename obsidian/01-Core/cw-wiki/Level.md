---
tags: [core, cw-wiki]
---

# Level

> `Level` enum defined in `cw_wiki::level` with 2 methods.

## Source

- `crates/cw-wiki/src/level.rs` — definition of `Level`

## How it works

1. `run` in [[codewiki]] and `security` in [[Api Module]] call `parse`.

## Items

Declared as `pub enum Level`.

Members:

- `pub fn as_str(self) -> &'static str` — public method, returns `&'static str`.
- `pub fn parse(text: &str) -> Option<Self>` — public method, returns `Option<Self>`; called by `run`, `security`.

## Used by

- [[Ci Module]] — from `CheckFinding`, `fails`, `to_junit`
- [[Security Module]] — from `Hit`, `SecurityFinding`
- [[codewiki]] — from `run`
- [[cw-wiki]] — from `scans_source_files_and_skips_tests`
- [[Api Module]] — from `security`

## See also

- [[_index]]
- [[Level Module]]

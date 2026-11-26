---
tags: [core, cw-wiki]
---

# Level

> `Level` enum defined in `cw_wiki::level` with 2 methods.

## What it does

How serious is a finding? `Level` answers with one of three values: note, warning or error. The analyses, the security scan and the CI report share it, so a threshold such as fail on warning means the same everywhere.

## Source

- `crates/cw-wiki/src/level.rs` — definition of `Level`

## How it works

`Level` orders note below warning below error, so `level >= threshold` is a valid test. `as_str` returns the lowercase word used in text, SARIF and JUnit output. `parse` turns that word back into a value; the command line and the API use it to read a threshold from the user.

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

## Gotchas

- `parse` is case sensitive and returns nothing for unknown words, so the caller must handle the missing value.
- Do not reorder the variants. The comparison depends on their order.

## See also

- [[_index]]
- [[Level Module]]

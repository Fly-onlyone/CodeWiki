---
tags: [core, cw-wiki]
---

# Report

> `Report` struct defined in `cw_wiki::generate` with 1 methods.

## Source

- `crates/cw-wiki/src/generate.rs` — definition of `Report`

## Items

Declared as `pub struct Report`.

Members:

- `fn summarize(&self) -> Vec<String>` — the lines of `Report::summary`, from what the run recorded.

## Depends on

- [[Generate Module]] — uses `KeptGone`, `Moved`, `notes`, `some_of`

## Used by

- [[cw-wiki]] — from `kept_gone`, `run`, `run_settled`, `sync`
- [[Generate Module]] — from `adopt_existing`, `carry_moves`, `generate`, `generate_with`
- [[Service Module]] — from `run_update`, `update`, `update_in`
- [[Api Module]] — from `run_update`, `update_repo`
- [[codewiki]] — from `print_report`

## See also

- [[_index]]
- [[Generate Module]]

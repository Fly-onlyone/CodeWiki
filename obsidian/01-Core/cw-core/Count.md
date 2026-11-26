---
tags: [core, cw-core]
---

# Count

> How many waiting notes a session explains.

## Source

- `crates/cw-core/src/auto_explain.rs` — definition of `Count`

## How it works

1. `load_count` in [[Auto Explain Module]] calls `parse`.

## Items

Declared as `pub enum Count`.

Members:

- `pub fn parse(text: &str) -> Option<Self>` — the count `text` names: a positive number or `all`.
- `pub fn of(self, waiting: usize) -> usize` — how many of `waiting` notes a session explains.

## Used by

- [[Auto Explain Module]] — from `DEFAULT`, `PRESETS`, `load_count`, `save_count`
- [[codewiki-app]] — from `AppView`, `read_auto_explain`
- [[Settings Pages Module]] — from `Model`, `count_choices`
- [[Briefing Module]] — from `Explain`, `explain_for`
- [[Live Module]] — from `waiting_page`
- [[Settings View Module]] — from `State`
- [[Action]] — from `Action`

## See also

- [[_index]]
- [[Auto Explain Module]]

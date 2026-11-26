---
tags: [core, cw-core]
---

# Count

> How many waiting notes a session explains.

## What it does

`Count` is how many waiting wiki notes a coding agent explains by itself at the start of a session, when auto explaining is on. It is either `Notes(n)`, at most n notes and never 0, or `All`. The app offers the presets 5, 10, 20, 50, 100 and all. The default is 20.

## Source

- `crates/cw-core/src/auto_explain.rs` — definition of `Count`

## How it works

The count is saved as one line in a small file in the data directory: a number or `all`. `load_count` reads it and calls `parse`. `parse` trims the text, accepts `all` in any case, and accepts a positive number. Anything else gives `None`, and `load_count` then returns the default. `save_count` writes the file through a temporary file and a rename, so a reader never sees half a file.

`of(waiting)` turns the count into a real number for a session. `Notes(n)` gives n or the number of waiting notes, whichever is less. `All` gives all of them. The [[Briefing Module]] uses it when it decides how large a batch to order at session start, and the settings page uses the presets.

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

## Gotchas

- Zero, a negative number, a word other than `all`, and text like `20 notes` all mean the default, 20. A broken file never makes a session explain nothing or everything.
- `save_count` writes at least 1 for `Notes`, so a file never holds 0.
- This is separate from the on or off switch. A broken switch file means off, but a broken count file means the default.

## See also

- [[_index]]
- [[Auto Explain Module]]

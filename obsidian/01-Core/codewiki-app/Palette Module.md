---
tags: [core, codewiki-app, module]
---

# Palette Module

> The command palette's entries and matching.

## What it does

No UI here.

## Source

- `crates/codewiki-app/src/palette.rs` — module `codewiki_app::palette`

## How it works

1. `AppView::palette_matches` in [[codewiki-app]] calls `Entry::new`.
2. `AppView::palette_matches` in [[codewiki-app]] calls `actions`. `actions` calls `Entry::new`.
3. `AppView::palette_matches` in [[codewiki-app]] calls `filter`. `filter` calls `rank`, then `Entry::new`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum Command` — what running a palette entry does.
- `pub struct Entry` — public struct; used by `AppView::palette_matches`, `actions`, `filter`.
- `pub fn actions() -> Vec<Entry>` — the fixed commands, in the order shown for an empty query.
- `fn rank(label: &str, words: &[String]) -> Option<usize>` — how well `label` matches the query words; lower is better, `None` is no match. Every word must occur. A match at the start of the label beats a match at the start of a word, which beats a match inside a word.
- `pub fn filter(entries: &[Entry], query: &str, limit: usize) -> Vec<Entry>` — entries matching `query`, best first, at most `limit`. Ties keep the order of `entries`. A non-empty query always ends with a full-text search entry, so Enter does something even when nothing matches.

## Depends on

- [[MotionPref]] — uses `ALL`, `MotionPref`
- [[Category]] — uses `ALL`, `Category`

## Used by

- [[codewiki-app]] — from `AppView`, `palette_matches`, `run_command`
- [[Icons Module]] — from `for_command`

## See also

- [[_index]]
- [[codewiki-app]]

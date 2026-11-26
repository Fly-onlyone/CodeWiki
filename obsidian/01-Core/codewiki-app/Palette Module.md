---
tags: [core, codewiki-app, module]
---

# Palette Module

> The command palette's entries and matching.

## What it does

This module is the data and the matching of the command palette, the Ctrl+K box that lets a user type a command, a note, a theme or a repository. It has no UI. It defines what each entry does (`Command`), what an entry looks like (`Entry`), the fixed list of commands (`actions`), and the search over a list of entries (`filter`). The app view draws the box and runs the chosen command.

## Source

- `crates/codewiki-app/src/palette.rs` — module `codewiki_app::palette`

## How it works

`AppView::palette_matches` in [[codewiki-app]] builds the full list. It starts with `actions`, which returns the fixed commands in the order shown for an empty query. Then come animation choices, one entry per [[MotionPref]], and one `Settings: <name>` entry per [[Category]], so typing a category name leads there. The app adds more entries of its own, such as notes, themes and repositories, and the command from `agents_entry`.

`agents_entry` looks at how the open project stands against the folders kept from coding agents. It offers `Keep agents out` when nothing covers the project, and `Let agents in` when an entry for exactly its folder does. It returns nothing when a parent folder covers it or when it holds an entry. Those are changed in Settings.

`filter` splits the query into words and keeps entries whose label contains every word. `rank` scores each one. A match at the start of the label wins over a match at the start of a word, which wins over a match inside a word. A match nearer the start also scores better. The lowest score comes first, and ties keep the order of the list. An empty query returns the first entries as they are.

## Items

- `pub enum Command` — what running a palette entry does.
- `pub struct Entry` — public struct; used by `AppView::palette_matches`, `actions`, `agents_entry`, `filter`.
- `pub fn agents_entry(kept: &crate::live::Kept) -> Option<Entry>` — the command that changes the list for the open project, judged by its registered folder. None when a parent entry covers it, or when it holds an entry: those are changed in Settings.
- `pub fn actions() -> Vec<Entry>` — the fixed commands, in the order shown for an empty query.
- `fn rank(label: &str, words: &[String]) -> Option<usize>` — how well `label` matches the query words; lower is better, `None` is no match. Every word must occur. A match at the start of the label beats a match at the start of a word, which beats a match inside a word.
- `pub fn filter(entries: &[Entry], query: &str, limit: usize) -> Vec<Entry>` — entries matching `query`, best first, at most `limit`. Ties keep the order of `entries`. A non-empty query always ends with a full-text search entry, so Enter does something even when nothing matches.

## Depends on

- [[MotionPref]] — uses `ALL`, `MotionPref`
- [[Category]] — uses `ALL`, `Category`
- [[Live Module]] — uses `Kept`

## Used by

- [[codewiki-app]] — from `AppView`, `palette_matches`, `run_command`
- [[Icons Module]] — from `for_command`

## Gotchas

- A non-empty query always ends with a `Search for` entry. It takes one slot of `limit`, so at most `limit` minus one matches show. This makes Enter do something even with no match.
- Every word of the query must occur in the label. Matching ignores case.
- Labels of the fixed commands must be unique. A test checks this.
- Some commands are listed only in some states: `Reconnect` only while the backend is unreachable, `InstallUpdate` only when there is an update, `Autostart` only for an installed copy. The code that adds them lives in the app view, not here.
- Adding a `Command` also needs a match arm in the app's `run_command` and, for an icon, in the Icons Module.

## See also

- [[_index]]
- [[codewiki-app]]

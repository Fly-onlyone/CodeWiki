---
tags: [core, codewiki-app, module]
---

# Panes Module

> The two side panes: whether each is shown.

## What it does

The sidebar and the details pane can each be hidden to leave the page more room. The choice is remembered between runs, one setting per pane.

## Source

- `crates/codewiki-app/src/panes.rs` — module `codewiki_app::panes`

## How it works

1. `AppView::new` in [[codewiki-app]] calls `load`. `load` calls `is_open`, then `read` in [[Prefs Module]].
2. `AppView::toggle_pane` in [[codewiki-app]] calls `store`. `store` calls `write` in [[Prefs Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const HIDDEN: &str` — what the setting of a hidden pane holds. Anything else, a missing setting included, means the pane is shown.
- `pub enum Pane` — A pane beside the page.
- `fn is_open(stored: Option<&str>) -> bool` — whether a pane whose setting holds `stored` is shown.
- `pub fn load(pane: Pane) -> bool` — whether `pane` was left shown.
- `pub fn store(pane: Pane, open: bool)` — remember whether `pane` is shown.

And 1 more private item with no comment.

## Depends on

- [[Prefs Module]] — uses `read`, `write`

## Used by

- [[codewiki-app]] — from `new`, `toggle_pane`

## See also

- [[_index]]
- [[codewiki-app]]

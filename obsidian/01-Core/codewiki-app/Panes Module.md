---
tags: [core, codewiki-app, module]
---

# Panes Module

> The two side panes: whether each is shown.

## What it does

The app has two side panes: the sidebar (the vault's notes, on the left) and the details pane (what is known about the page, on the right). Either can be hidden to give the page more room. This module remembers each choice between runs, one setting per pane, stored through [[Prefs Module]].

## Source

- `crates/codewiki-app/src/panes.rs` — module `codewiki_app::panes`

## How it works

`store` writes the word shown or hidden to the setting named after the pane (sidebar or details). `load` reads that setting back through `is_open`. The app calls `load` once at start (`AppView::new`) and `store` each time the user toggles a pane (`AppView::toggle_pane`).

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

## Gotchas

- Shown is the default. Only the exact word hidden (surrounding whitespace is trimmed) hides a pane. A missing file, an empty file, or text like off, false, 0 or Hidden with a capital H leaves it shown.
- A failed write is ignored by [[Prefs Module]], so a pane can reappear after a restart without any error.

## See also

- [[_index]]
- [[codewiki-app]]

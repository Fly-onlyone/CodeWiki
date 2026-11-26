---
tags: [core, codewiki-app, module]
---

# Tab Ops Module

> Which tabs stay when some are closed: the logic of the tab menu, kept apart from the view so that it can be tested.

## Source

- `crates/codewiki-app/src/tab_ops.rs` — module `codewiki_app::tab_ops`

## How it works

1. `AppView::context_items` in [[codewiki-app]] calls `offered`. `offered` calls `plan`.
2. `AppView::close_tabs` in [[codewiki-app]] calls `plan`.

## Items

- `pub enum Close` — which tabs to close, named by the tab the user acted on.
- `pub fn offered(count: usize, index: usize) -> Vec<Close>` — the ways to close tabs that the menu of the tab at `index`, of `count`, offers: those that close something.
- `pub struct Outcome` — what is left after closing.
- `pub fn plan(count: usize, active: usize, close: Close) -> Option<Outcome>` — what closing does to `count` tabs of which `active` is shown. `None` when it would close nothing, or the last tab by itself.

## Used by

- [[codewiki-app]] — from `ContextCommand`, `close_tabs`, `context_items`

## See also

- [[_index]]
- [[codewiki-app]]

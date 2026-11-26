---
tags: [core, codewiki-app, module]
---

# Tab Ops Module

> Which tabs stay when some are closed: the logic of the tab menu, kept apart from the view so that it can be tested.

## What it does

When a user closes tabs from the tab menu (Close, Close others, Close to the right, Close to the left, Close all), the app must know which tabs stay and which one shows afterwards. This module answers that with pure functions on indexes. It has no UI, so a test can check every case without a window. `AppView::context_items` asks it which menu rows to offer, and `AppView::close_tabs` asks it what the result of a close is.

## Source

- `crates/codewiki-app/src/tab_ops.rs` — module `codewiki_app::tab_ops`

## How it works

`Close` names the way to close, with the index of the tab the user acted on. `plan(count, active, close)` returns an `Outcome`: `keep`, the indexes of the tabs that stay in their old order, `active`, the shown tab as an index into `keep`, and `active_closed`, true when the shown tab was closed so another tab's page must show.

The active tab after a close follows three rules. If the active tab stays, it stays shown. If `One` closes the active tab, the tab that took its place shows, or the last one. For Others, Right and Left, the tab the user acted on stays and shows.

`offered(count, index)` builds the menu. It tries every `Close` for that tab and keeps those for which `plan` returns something, in the fixed order One, Others, Right, Left, All. So the menu lists only rows that do something.

## Items

- `pub enum Close` — which tabs to close, named by the tab the user acted on.
- `pub fn offered(count: usize, index: usize) -> Vec<Close>` — the ways to close tabs that the menu of the tab at `index`, of `count`, offers: those that close something.
- `pub struct Outcome` — what is left after closing.
- `pub fn plan(count: usize, active: usize, close: Close) -> Option<Outcome>` — what closing does to `count` tabs of which `active` is shown. `None` when it would close nothing, or the last tab by itself.

## Used by

- [[codewiki-app]] — from `ContextCommand`, `close_tabs`, `context_items`

## Gotchas

- `plan` returns `None` when nothing would close, when the index is out of range, and when it would close the last tab by itself. With one tab, only `All` is offered.
- `All` keeps nothing, even of one tab. The caller must make a new tab. The comment on `All` says the app keeps one tab.
- `keep` holds indexes from before the close, not after. Use `active` as an index into `keep`, not into the old list.
- `active_closed` is how the caller knows to leave the old page. Ignoring it shows a page of a closed tab.

## See also

- [[_index]]
- [[codewiki-app]]

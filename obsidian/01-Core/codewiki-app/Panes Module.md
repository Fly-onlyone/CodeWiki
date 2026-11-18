---
tags: [core, codewiki-app, module]
---

# Panes Module

> The two side panes: whether each is shown.

## Source

- `crates/codewiki-app/src/panes.rs` — module `codewiki_app::panes`

## How it works

Main items:

- `Pane` — A pane beside the page.
- `store` — remember whether `pane` is shown.
- `load` — whether `pane` was left shown.
- `HIDDEN` — what the setting of a hidden pane holds.
- `is_open` — whether a pane whose setting holds `stored` is shown.
- `SHOWN` — const

## Depends on

- [[Prefs Module]] — uses `read`, `write`

## Used by

- [[codewiki-app]] — from `new`, `toggle_pane`

## See also

- [[_index]]
- [[codewiki-app]]

---
tags: [core, codewiki-app, module]
---

# Base Module

> What every pressable control shares: MUI's `ButtonBase`.

## Source

- `crates/codewiki-app/src/mui/base.rs` — module `codewiki_app::mui::base`

## How it works

Main items:

- `Touch` — the remembered state of one control.
- `Base` — the state of one control for this frame, and the means to wire it up.
- `OnClick` — what a click does.
- `Hover` — whether the pointer is over an element that is not pressable, such as a text field or a card.
- `Ink` — how a control's ripple is drawn.
- `focus_ring` — the ring a control shows while it has keyboard focus.

## Depends on

- [[Ripple Module]] — uses `Point`, `Wave`, `finished`, `moving`

## Used by

- [[Button Module]] — from `IconButton`, `render`
- [[Select Module]] — from `Select`, `render`
- [[Button]] — from `Button`, `render`
- [[ListItemButton]] — from `ListItemButton`, `render`
- [[Doc View Module]] — from `table_row`
- [[Switch Module]] — from `render`
- [[Tabs Module]] — from `render`
- [[Text Field Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]

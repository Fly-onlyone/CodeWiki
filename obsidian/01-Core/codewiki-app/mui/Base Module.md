---
tags: [core, codewiki-app, module]
---

# Base Module

> What every pressable control shares: MUI's `ButtonBase`.

## What it does

A control built on `Base` knows whether the pointer is over it, whether it is pressed and whether it has keyboard focus; it draws a ripple when pressed; and Tab reaches it and Enter or Space press it.

GPUI switches `.hover()` and `.active()` styles instantly. So that a control can ease between its states, the state is kept here and the control reads it while it renders.

## Source

- `crates/codewiki-app/src/mui/base.rs` — module `codewiki_app::mui::base`

## How it works

1. `Button::render` and `IconButton::render` in [[Button Module]], and 5 more, call `Base::new`.
2. `Button::render` and `IconButton::render` in [[Button Module]], and 5 more, call `focus_ring`. `focus_ring` calls `px` in [[Zoom Module]].
3. `table_row` in [[Doc View Module]] and `TextField::render` in [[Text Field Module]] call `Hover::new`.

## Items

- `pub type OnClick = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>` — what a click does.
- `pub struct Touch` — the remembered state of one control.
- `pub struct Ink` — how a control's ripple is drawn.
- `pub struct Base` — the state of one control for this frame, and the means to wire it up.
- `pub fn focus_ring(primary: Hsla) -> BoxShadow` — the ring a control shows while it has keyboard focus. TradingAgent's `focusRing`: `0 0 0 2px rgba(primary, 0.4)`.
- `pub struct Hover` — whether the pointer is over an element that is not pressable, such as a text field or a card.

## Depends on

- [[Ripple Module]] — uses `Point`, `Wave`, `finished`, `moving`
- [[Zoom Module]] — uses `px`

## Used by

- [[Button Module]] — from `Button`, `IconButton`, `render`
- [[Select Module]] — from `Select`, `render`
- [[ListItemButton]] — from `ListItemButton`, `render`
- [[Doc View Module]] — from `table_row`
- [[Switch Module]] — from `render`
- [[Tabs Module]] — from `render`
- [[Text Field Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]

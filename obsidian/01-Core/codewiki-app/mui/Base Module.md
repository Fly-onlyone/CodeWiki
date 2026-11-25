---
tags: [core, codewiki-app, module]
---

# Base Module

> What every pressable control shares: MUI's `ButtonBase`.

## What it does

The shared core of every pressable control, like MUI's `ButtonBase`. A control calls `Base::new` while it renders and gets its state for this frame: `hovered`, `pressed`, `focused`, and `press`, a 0 to 1 value for how far it is into its pressed look. `Base::attach` then wires pointer, keyboard focus and click handling onto the element and paints the ripple over it.

GPUI switches hover and active styles instantly. Keeping the state here lets a control ease between looks instead. `Hover` is a small sibling for things that are not pressable, such as a text field or a table row.

## Source

- `crates/codewiki-app/src/mui/base.rs` — module `codewiki_app::mui::base`

## How it works

State lives in a `Touch` entity keyed on the control's id: hover, pressed, the running ripple waves and a focus handle. `Base::new` drops finished waves, and asks for another animation frame while a wave still moves (unless the user reduces motion).

`press` is a transition toward 1 while pressed and toward 0 otherwise. Going in it is fast (`PRESS_IN`), going out slow (`PRESS_OUT`), so a short click still shows its pressed look before it fades. Controls read `press` to fill, darken or sink; see [[Interaction Module]] for the strengths.

`attach` adds the handlers. Hover sets the flag and releases on leave. Mouse down stops propagation, prevents default (so a mouse press does not take keyboard focus), and starts a wave at the pointer. Mouse up or up-outside releases it. A keyboard click starts and releases a centred wave at once. A canvas child paints the waves with the ripple opacities from the knobs (see [[Ripple Module]]). With reduced motion it paints a plain tint while pressed.

## Items

- `pub type OnClick = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>` — what a click does.
- `pub struct Touch` — the remembered state of one control.
- `pub struct Ink` — how a control's ripple is drawn.
- `pub struct Base` — the state of one control for this frame, and the means to wire it up.
- `pub fn focus_ring(primary: Hsla) -> BoxShadow` — the ring a control shows while it has keyboard focus. TradingAgent's `focusRing`: `0 0 0 2px rgba(primary, 0.4)`.
- `pub struct Hover` — whether the pointer is over an element that is not pressable, such as a text field or a card.

## Depends on

- [[Ripple Module]] — uses `Point`, `Wave`, `finished`, `moving`
- [[Motion Module (mui)]] — uses `PRESS_IN`, `PRESS_OUT`, `scaled`
- [[Curve Module]] — uses `EASE_IN_OUT`, `EASE_OUT`
- [[Interaction Module]] — uses `current`
- [[Tokens Module]] — uses `easing`
- [[Review Module]] — uses `retain`
- [[Zoom Module]] — uses `px`

## Used by

- [[Button Module]] — from `Button`, `IconButton`, `render`
- [[Checkbox Module]] — from `Checkbox`, `render`
- [[List Module]] — from `ListItemButton`, `render`
- [[Select Module]] — from `Select`, `render`
- [[codewiki-app]] — from `toasts_view`
- [[Doc View Module]] — from `table_row`
- [[Snackbar Module]] — from `Content`
- [[Switch Module]] — from `render`

## Gotchas

- The press transition uses the name `touch-press`, not `press`, because `Button` keeps its own glow state under `press` for the same id.
- The focus handle is made with `tab_index(0)` and `tab_stop(true)`. GPUI reads those from a tracked handle, not from the element. Without them no control was a tab stop and Tab stayed in the search box.
- A disabled control gets no handlers and no ripple, and its hover and press state is cleared.
- Mouse down stops propagation so a control inside another, such as a tab's close button, takes the press alone.
- `focused` is only keyboard focus. A mouse press does not give it.

## See also

- [[_index]]
- [[codewiki-app]]

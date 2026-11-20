---
tags: [core, codewiki-app]
---

# Enter

> An entrance: where the element comes from, on which spring, after what wait.

## Source

- `crates/codewiki-app/src/mui/motion.rs` — definition of `Enter`

## How it works

1. `column` and `draw_strip` in [[Doc View Module]] call `card`. `card` calls `rise`, then `after`, then `stagger_delay` in [[Motion Module (mui)]].
2. `table_row` in [[Doc View Module]] calls `row`. `row` calls `slide`, then `after`, then `stagger_delay` in [[Motion Module (mui)]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct Enter`.

Members:

- `pub const PAGE: Self` — `pageEntrance`.
- `pub const TAB: Self` — `tabSwitch`.
- `pub const DIALOG: Self` — `scaleIn`, without the scale: a short drop on the bouncy spring.
- `pub const MENU: Self` — A menu under its button. MUI grows a menu out of its anchor (`Grow/Grow.js`), which is a scale; this is the drop that is left.
- `pub const fn rise(distance: f32, spring: Spring) -> Self` — comes up from `distance` pixels below.
- `pub const fn slide(distance: f32, spring: Spring) -> Self` — comes in from `distance` pixels to the left.
- `pub fn card(index: usize) -> Self` — item `index` of a `staggerCascade`.
- `pub fn row(index: usize) -> Self` — item `index` of a `listStagger`.
- `pub const fn after(mut self, delay: Duration) -> Self` — public method, returns `Self`; called by `Enter::card`, `Enter::row`.
- `pub fn at(self, progress: f32) -> (f32, f32, f32)` — where the element is and how visible, `progress` of the way in. `progress` may pass 1 while the spring overshoots; the offset follows it and the opacity does not.

## Depends on

- [[Motion Module (mui)]] — uses `BOUNCY`, `CARD_RISE`, `CARD_STAGGER`, `PAGE`
- [[Spring]] — uses `Spring`

## Used by

- [[Doc View Module]] — from `column`, `draw_strip`, `render`, `table_row`
- [[codewiki-app]] — from `diagram_viewer_view`, `page_content`, `palette_view`
- [[Illustration Module]] — from `tabs`
- [[Menu Module]] — from `render`
- [[Motion Module (mui)]] — from `enter`

## See also

- [[_index]]
- [[Motion Module (mui)]]

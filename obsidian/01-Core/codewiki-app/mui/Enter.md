---
tags: [core, codewiki-app]
---

# Enter

> An entrance: where the element comes from, on which spring, after what wait.

## Source

- `crates/codewiki-app/src/mui/motion.rs` — definition of `Enter`

## How it works

Declared as `pub struct Enter`.

Public API:

- `pub const fn rise(distance: f32, spring: Spring) -> Self` — comes up from `distance` pixels below.
- `pub fn card(index: usize) -> Self` — item `index` of a `staggerCascade`.
- `pub const fn slide(distance: f32, spring: Spring) -> Self` — comes in from `distance` pixels to the left.
- `pub const PAGE: Self = Self::rise(PAGE_RISE, PAGE)` — `pageEntrance`.
- `pub fn row(index: usize) -> Self` — item `index` of a `listStagger`.
- `pub fn at(self, progress: f32) -> (f32, f32, f32)` — where the element is and how visible, `progress` of the way in.

## Depends on

- [[Motion Module (mui)]] — uses `BOUNCY`, `CARD_RISE`, `CARD_STAGGER`, `PAGE`
- [[Spring]] — uses `Spring`

## Used by

- [[Doc View Module]] — from `column`, `draw_strip`, `render`, `table_row`
- [[codewiki-app]] — from `palette_view`, `viewer`
- [[Illustration Module]] — from `tabs`
- [[Menu Module]] — from `render`
- [[Motion Module (mui)]] — from `enter`

## See also

- [[_index]]
- [[Motion Module (mui)]]

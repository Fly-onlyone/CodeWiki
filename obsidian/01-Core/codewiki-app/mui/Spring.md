---
tags: [core, codewiki-app]
---

# Spring

> A spring as Framer Motion describes one.

## Source

- `crates/codewiki-app/src/mui/motion.rs` — definition of `Spring`

## How it works

1. `BOUNCY` and `GENTLE` in [[Motion Module (mui)]], and 2 more, call `new`.

## Items

Declared as `pub struct Spring`.

Members:

- `pub const fn new(stiffness: f32, damping: f32) -> Self` — public method, returns `Self`; called by `BOUNCY`, `GENTLE`, `PAGE`, `SNAPPY`.
- `pub const fn mass(mut self, mass: f32) -> Self` — public method, returns `Self`.
- `pub fn response(self) -> Duration` — the time one swing would take with no damping: `2π / √(k/m)`.
- `pub fn damping_ratio(self) -> f32` — the damping ratio `c / (2√(km))`. Below 1 the spring overshoots.
- `fn slowed(self, factor: f32) -> Self` — the same motion, `factor` times slower.
- `pub fn config(self) -> SpringConfig` — for `with_spring` and `sampled_easing`.
- `pub fn animation(self) -> Animation` — one run of the spring from rest at 0 to rest at 1, as an animation.
- `pub fn policy(self, epsilon: f32) -> kit::Spring` — for `gpui_kit::base::motion::spring`, which follows a moving target. `epsilon` is how close counts as arrived, in the unit of the value.

## Depends on

- [[Motion Module (mui)]] — uses `EPSILON`, `scale`, `scaled`
- [[Tokens Module]] — uses `easing`

## Used by

- [[Motion Module (mui)]] — from `BOUNCY`, `GENTLE`, `PAGE`, `SNAPPY`
- [[Enter]] — from `Enter`, `rise`, `slide`
- [[Doc View Module]] — from `stat_tile`
- [[Button Module]] — from `render`
- [[Tabs Module]] — from `render`
- [[ListItemButton]] — from `render`

## See also

- [[_index]]
- [[Motion Module (mui)]]

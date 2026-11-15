---
tags: [core, codewiki-app]
---

# Spring

> A spring as Framer Motion describes one.

## Source

- `crates/codewiki-app/src/mui/motion.rs` — definition of `Spring`

## How it works

Declared as `pub struct Spring`.

Public API:

- `pub fn config(self) -> SpringConfig` — for `with_spring` and `sampled_easing`.
- `pub fn response(self) -> Duration` — the time one swing would take with no damping: `2π / √(k/m)`.
- `pub fn damping_ratio(self) -> f32` — the damping ratio `c / (2√(km))`.
- `pub fn policy(self, epsilon: f32) -> kit::Spring` — for `gpui_kit::base::motion::spring`, which follows a moving target.
- `pub fn animation(self) -> Animation` — one run of the spring from rest at 0 to rest at 1, as an animation.
- `pub const fn new(stiffness: f32, damping: f32) -> Self`

## Depends on

- [[Motion Module (mui)]] — uses `EPSILON`, `scale`, `scaled`

## Used by

- [[Motion Module (mui)]] — from `BOUNCY`, `GENTLE`, `PAGE`, `SNAPPY`
- [[Enter]] — from `Enter`, `rise`, `slide`
- [[Doc View Module]] — from `stat_tile`
- [[Tabs Module]] — from `render`
- [[Button]] — from `render`
- [[ListItemButton]] — from `render`

## See also

- [[_index]]
- [[Motion Module (mui)]]

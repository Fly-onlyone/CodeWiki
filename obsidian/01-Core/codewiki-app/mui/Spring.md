---
tags: [core, codewiki-app]
---

# Spring

> A spring as Framer Motion describes one.

## What it does

A spring described the way Framer Motion does it: stiffness, damping and mass. The presets `SNAPPY`, `BOUNCY`, `GENTLE` and `PAGE` in [[Motion Module (mui)]] are values of this type. It turns that description into what GPUI and gpui-kit need, either a fixed animation or a spring that follows a moving target.

## Source

- `crates/codewiki-app/src/mui/motion.rs` — definition of `Spring`

## How it works

`new` and `mass` build a spring in const context. `response` is the time of one undamped swing, and `damping_ratio` says whether it overshoots (below 1 it does). `slowed` makes the same motion slower by a factor: stiffness divides by the factor squared and damping by the factor, so the damping ratio stays and only the speed changes.

`config` applies the global motion scale through `slowed` and returns a GPUI `SpringConfig`. `animation` samples that config from rest at 0 to rest at 1 into an `Animation`, for entrances. `policy` makes a gpui-kit spring from the response time (scaled) and damping ratio, with an epsilon for when it counts as arrived. Controls use `policy` for springs that chase a changing target, such as a row's hover or the selected bar.

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

- [[Motion Module (mui)]] — from `BOUNCY`, `Enter`, `GENTLE`, `PAGE`
- [[Doc View Module]] — from `stat_tile`
- [[Button Module]] — from `render`
- [[Interaction Module]] — from `Interaction`
- [[List Module]] — from `render`
- [[Tabs Module]] — from `render`

## Gotchas

- `policy` takes an epsilon in the unit of the value it follows. Use a small one (0.001) for 0..1 values and a larger one for pixels.
- All three outputs honor the motion scale. A spring made by hand from `SpringConfig` would not.
- `BOUNCY` has damping ratio under 0.7, so it visibly overshoots. Code that uses its value as a length or height must clamp, as the row bar does.

## See also

- [[_index]]
- [[Motion Module (mui)]]

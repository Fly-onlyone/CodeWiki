---
tags: [core, codewiki-app, module]
---

# Ripple Module

> The ripple a control answers a press with.

## What it does

Follows MUI's `ButtonBase/TouchRipple.js`: a disc grows from the point of the press to cover the control, and fades once the press ends.

GPUI clips to rectangles only, so a disc inside a control with rounded corners would show beyond them. Each ripple is therefore painted as the shape it should have: the disc intersected with the control's rounded rectangle. Both are convex, so the intersection is one polygon.

Lengths are in pixels and points are relative to the control's top-left corner.

## Source

- `crates/codewiki-app/src/mui/ripple.rs` — module `codewiki_app::mui::ripple`

## How it works

1. `Touch::press` in [[Base Module]] calls `Wave::new`.
2. `Base::attach` in [[Base Module]] calls `paint`. `paint` calls `px` in [[Zoom Module]], then `rounded_rect`, then `origin`, then `size`, then `covers`, then `clip`, and 1 more.
3. `Base::attach` in [[Base Module]] calls `paint_pressed`. `paint_pressed` calls `px` in [[Zoom Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const DURATION: Duration` — `TouchRipple.js`, `DURATION`: the time a ripple takes to grow, and to fade.
- `const ENTER_OPACITY: (f32, f32)` — `enterKeyframe`: the opacity a ripple starts and ends its growth at.
- `const DISC_SIDES: usize` — sides of the polygon that stands in for the disc.
- `pub type Point = (f32, f32)` — public type alias; used by `Touch::press`, `Wave`, `Wave::new`, `clip` and 5 more.
- `pub fn origin(width: f32, height: f32, press: Option<Point>, centered: bool) -> Point` — where a ripple starts. `TouchRipple.js`, `start`: the middle of the control for a centered ripple or a press without a position, else the point pressed.
- `pub fn size(width: f32, height: f32, origin: Point, centered: bool) -> f32` — the diameter a ripple grows to. `TouchRipple.js`, `rippleSize`: wide enough to reach the corner farthest from `origin`; a centered ripple is smaller, and leaves the corners of a square control clear.
- `pub struct Wave` — one ripple: when the press began and, once it has, when it ended.
- `fn frame(growing: Duration, fading: Option<Duration>) -> (f32, f32)` — growth and opacity of a ripple that began `growing` ago and whose press ended `fading` ago. Growth and fade run side by side, as in MUI, where a quick click starts the fade while the disc is still growing.
- `pub fn rounded_rect(width: f32, height: f32, radius: f32) -> Vec<Point>` — the outline of a `width` by `height` rectangle with corners of `radius`, as a convex polygon.
- `pub fn disc(centre: Point, radius: f32) -> Vec<Point>` — A disc as a polygon, wound the same way as `rounded_rect`.
- `pub fn clip(subject: &[Point], bounds: &[Point]) -> Vec<Point>` — the part of `subject` inside the convex polygon `bounds` (Sutherland and Hodgman). Both must be wound as `rounded_rect` is.
- `pub fn covers(centre: Point, radius: f32, width: f32, height: f32) -> bool` — whether a disc covers the whole `width` by `height` rectangle.
- `paint` — paint `waves` over a control at `bounds` whose corners have `radius`. Call while painting. `centered` ripples start from the middle, as on an icon button.
- `pub fn paint_pressed(bounds: Bounds<Pixels>, radius: f32, color: Hsla, window: &mut Window)` — the tint a pressed control shows in place of a ripple when motion is reduced: the ripple's resting opacity over the whole control.

## Depends on

- [[Motion Module (mui)]] — uses `scaled`
- [[Tokens Module]] — uses `easing`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Zoom Module]] — uses `px`

## Used by

- [[Base Module]] — from `Touch`, `attach`, `new`, `press`

## See also

- [[_index]]
- [[codewiki-app]]

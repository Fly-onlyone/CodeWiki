---
tags: [core, codewiki-app, module]
---

# Ripple Module

> The ripple a control answers a press with.

## What it does

Draws the ripple that answers a press, after MUI's `TouchRipple`: a disc grows from the pressed point until it covers the control, then fades when the press ends.

GPUI can clip only to rectangles. A disc in a control with rounded corners would poke out past the curve. So each ripple is painted as a polygon: the disc cut to the control's rounded rectangle. Both shapes are convex, so the cut is one polygon. [[Base Module]] owns the waves and calls `paint` for them.

## Source

- `crates/codewiki-app/src/mui/ripple.rs` — module `codewiki_app::mui::ripple`

## How it works

A press creates a `Wave` with its start time and optional press point. A release stamps the end time. `Wave::frame` gives two numbers: growth (0 to 1) and opacity. Growth and fade run side by side over 550 ms, scaled by the motion setting, both on the ease-in-out curve. Opacity goes from the first to the second ripple opacity as the disc grows, then is multiplied by what is left of the fade. The opacities are passed in; the app takes them from [[Interaction Module]].

`paint` turns bounds into control-local coordinates, builds the rounded outline with `rounded_rect`, and for each wave finds the start with `origin` and the final diameter with `size`. If the disc already covers all four corners (`covers`), it paints one rounded quad. Otherwise it clips `disc` against the outline with `clip` (Sutherland and Hodgman) and paints the polygon.

`Wave::moving` tells the caller whether another frame is needed. A ripple held at full size is still. `finished` says when a faded wave can be dropped.

`paint_pressed` is the reduced-motion form: a plain tint over the whole control.

## Items

- `pub const DURATION: Duration` — `TouchRipple.js`, `DURATION`: the time a ripple takes to grow, and to fade.
- `const ENTER_OPACITY: (f32, f32)` — `enterKeyframe`: the opacity a ripple starts and ends its growth at. The app's controls take theirs from `interaction`; this is MUI's.
- `const DISC_SIDES: usize` — sides of the polygon that stands in for the disc.
- `pub type Point = (f32, f32)` — public type alias; used by `Touch::press`, `Wave`, `Wave::new`, `clip` and 5 more.
- `pub fn origin(width: f32, height: f32, press: Option<Point>, centered: bool) -> Point` — where a ripple starts. `TouchRipple.js`, `start`: the middle of the control for a centered ripple or a press without a position, else the point pressed.
- `pub fn size(width: f32, height: f32, origin: Point, centered: bool) -> f32` — the diameter a ripple grows to. `TouchRipple.js`, `rippleSize`: wide enough to reach the corner farthest from `origin`; a centered ripple is smaller, and leaves the corners of a square control clear.
- `pub struct Wave` — one ripple: when the press began and, once it has, when it ended.
- `fn frame(growing: Duration, fading: Option<Duration>, opacity: (f32, f32)) -> (f32, f32)` — growth and opacity of a ripple that began `growing` ago and whose press ended `fading` ago. Growth and fade run side by side, as in MUI, where a quick click starts the fade while the disc is still growing.
- `pub fn rounded_rect(width: f32, height: f32, radius: f32) -> Vec<Point>` — the outline of a `width` by `height` rectangle with corners of `radius`, as a convex polygon.
- `pub fn disc(centre: Point, radius: f32) -> Vec<Point>` — A disc as a polygon, wound the same way as `rounded_rect`.
- `pub fn clip(subject: &[Point], bounds: &[Point]) -> Vec<Point>` — the part of `subject` inside the convex polygon `bounds` (Sutherland and Hodgman). Both must be wound as `rounded_rect` is.
- `pub fn covers(centre: Point, radius: f32, width: f32, height: f32) -> bool` — whether a disc covers the whole `width` by `height` rectangle.
- `paint` — paint `waves` over a control at `bounds` whose corners have `radius`. Call while painting. `centered` ripples start from the middle, as on an icon button; `opacity` is the ripple's at its start and once grown.
- `paint_pressed` — the tint a pressed control shows in place of a ripple when motion is reduced: the ripple's resting opacity, `opacity`, over the whole control.

## Depends on

- [[Motion Module (mui)]] — uses `scaled`
- [[Tokens Module]] — uses `easing`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Zoom Module]] — uses `px`

## Used by

- [[Base Module]] — from `Touch`, `attach`, `new`, `press`

## Gotchas

- `clip` needs both polygons wound the same way, as `rounded_rect` does. A disc from `disc` matches it.
- A centered ripple gets a smaller diameter than the corner reach, so it leaves the corners of a square control clear, as MUI does.
- The `ENTER_OPACITY` constant exists only for tests. The shipped opacities come from the interaction knobs, not from MUI's 0.1 and 0.3.
- A disc outside the control clips to nothing and is skipped, as is one with fewer than three points.
- A wave released twice keeps its first release time.

## See also

- [[_index]]
- [[codewiki-app]]

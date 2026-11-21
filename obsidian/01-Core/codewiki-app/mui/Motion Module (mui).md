---
tags: [core, codewiki-app, module]
---

# Motion Module (mui)

> How things move: springs, entrances and stagger.

## What it does

The springs are the ones TradingAgent gives Framer Motion (`frontend/src/utils/animations/variants.ts`). Framer describes a spring by stiffness, damping and mass, and so does GPUI, so the values carry over as they are.

GPUI cannot scale an element, so the entrances here keep Framer's fade and travel and leave out its scale.

Set `CODEWIKI_MOTION_SCALE` to a number above 1 to stretch every motion by that factor. A window capture can then catch an animation half way.

## Source

- `crates/codewiki-app/src/mui/motion.rs` — module `codewiki_app::mui::motion`

## How it works

1. `AppView::render` and `AppView::forget_saved_note` in [[codewiki-app]], and 15 more, call `scaled`. `scaled` calls `scale`.
2. `Spring::config` in [[Spring]] calls `scale`. `scale` calls `parse_scale`.
3. `AppView::render` in [[codewiki-app]] calls `drawer`. `drawer` calls `scaled`, then `easing` in [[Tokens Module]].
4. `stat_tile` in [[Doc View Module]] calls `count_up`.
5. `column` and `draw_strip` in [[Doc View Module]] call `Enter::card`. `Enter::card` calls `Enter::rise`, then `Enter::after`, then `stagger_delay`.

1 more entry point: `Enter::row`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- [[Spring]] — A spring as Framer Motion describes one.
- `pub const SNAPPY: Spring` — `spring.snappy`: the default for things that enter or follow the pointer.
- `pub const BOUNCY: Spring` — `spring.bouncy`: overshoots; for dialogs and small accents.
- `pub const PAGE: Spring` — the spring of `pageEntrance` and `scrollReveal`.
- `pub const PAGE_RISE: f32` — travel of `pageEntrance`, in pixels.
- `pub const CARD_RISE: f32` — travel of a `staggerCascade` item.
- `pub const ROW_SLIDE: f32` — travel of a `listStagger` item, from the left.
- `pub const CARD_STAGGER: Duration` — `staggerCascade`: time between two cards.
- `pub const ROW_STAGGER: Duration` — `listStagger`: time between two rows.
- `pub const STAGGER_CAP: usize` — items after this many enter together with the last staggered one. Without a cap the last row of a long table would arrive seconds late.
- `pub const EXIT: Duration` — `scaleIn` exit, and the exit of `tabSwitch` and `pageEntrance`.
- `pub const NOTE_HOLD: Duration` — how long the word "Saved" stays beside a setting. Not a motion of MUI or of TradingAgent: long enough to be read, and gone before the next change.
- `pub const LEVEL_HOLD: Duration` — how long the zoom level stays in the title bar after a change. Not a motion of MUI or of TradingAgent: long enough to read the figure while the wheel turns, and gone soon after it stops.
- `pub fn scale() -> f32` — the factor every motion is stretched by; 1 unless `CODEWIKI_MOTION_SCALE` says otherwise.
- `pub fn scaled(duration: Duration) -> Duration` — `duration` stretched by `scale`.
- `drawer` — how much of a side pane shows, from 0 (hidden) to 1 (shown), on the way to `open`: MUI's persistent `Drawer`. It enters over `enteringScreen` on `easeOut` and leaves over `leavingScreen` on `sharp` (`Drawer/Drawer.js`, `Slide/Slide.js`). A pane that starts hidden starts at 0, with no motion.
- `pub fn count_up(value: &str, progress: f32) -> String` — A figure such as `286`, `62.5%` or `1,204 lines`, `progress` of the way up from zero: TradingAgent's `RollingDigits`. Text that does not start with a number is returned as it is.
- `pub fn stagger_delay(index: usize, step: Duration) -> Duration` — how long item `index` of a staggered group waits before it enters.
- `pub struct Enter` — an entrance: where the element comes from, on which spring, after what wait.
- `pub trait EnterExt: Styled + IntoElement + Sized + 'static` — elements that can make an entrance.

And 3 more items.

## Depends on

- [[Spring]] — uses `Spring`, `animation`, `mass`, `new`
- [[Curve Module]] — uses `EASE_OUT`, `SHARP`
- [[Duration Module]] — uses `ENTERING_SCREEN`, `LEAVING_SCREEN`
- [[Tokens Module]] — uses `easing`
- [[Zoom Module]] — uses `px`

## Used by

- [[codewiki-app]] — from `check_for_update`, `connect`, `diagram_viewer_view`, `draw_queued_diagrams`
- [[Doc View Module]] — from `column`, `draw_strip`, `line`, `render`
- [[Ripple Module]] — from `finished`, `frame`, `moving`
- [[Spring]] — from `animation`, `config`, `policy`
- [[Button Module]] — from `ease`, `render`
- [[Feedback Module]] — from `linear_progress`
- [[List Module]] — from `render`
- [[Menu Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]

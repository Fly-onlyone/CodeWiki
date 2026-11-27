---
tags: [core, codewiki-app, module]
---

# Motion Module (mui)

> How things move: springs, entrances and stagger.

## What it does

Holds how things move in the app: the spring presets, entrances, stagger delays, the press timings, and how long a few short messages stay on screen. The springs are TradingAgent's Framer Motion presets. Framer and GPUI both describe a spring by stiffness, damping and mass, so the numbers carry over unchanged.

GPUI cannot scale an element, so entrances keep Framer's fade and travel and drop its scale. Set `CODEWIKI_MOTION_SCALE` to a number of 1 or more to stretch every motion. A window capture can then catch an animation half way.

## Source

- `crates/codewiki-app/src/mui/motion.rs` — module `codewiki_app::mui::motion`

## How it works

`scale` reads the environment variable once and keeps it. `parse_scale` accepts only finite numbers of at least 1, so motion is never sped up and bad text means 1. `scaled` stretches a duration by it, and `Spring::config` and `Spring::policy` apply it to springs (see [[Spring]]). Controls call `scaled` for every transition they build.

The spring presets are `SNAPPY`, `BOUNCY`, `GENTLE` and `PAGE`. `Enter` describes an entrance: a start offset, a spring and a delay. `Enter::PAGE`, `DIALOG`, `MENU` and `TOAST` are fixed ones. `Enter::card` rises from below and `Enter::row` slides in from the left. Both add a stagger from `stagger_delay`, which stops growing after `STAGGER_CAP` (12) items. `EnterExt::enter` runs two animations: a hold for the delay, then the spring. It runs again whenever the element's id changes. `Enter::at` turns progress into offset and opacity. Opacity is clamped, but the offset follows an overshoot.

`drawer` gives a side pane's openness from 0 to 1: ease-out over the entering duration, sharp over the leaving duration. `count_up` shows a figure such as `62.5%` part way up from zero.

`glide` is the clock of the outline's scroll. The caller passes the time since the glide began, and gets back a number from 0 to 1 over MUI's standard duration, stretched by `scaled`, on an ease-in-out curve. It returns exactly 1 once the time is up, so the caller can tell the glide has arrived. It holds no state: the app's `step_outline` asks it each frame and maps the result onto the scroll position (see [[Outline Module]]).

`PRESS_IN` (50 ms) and `PRESS_OUT` (250 ms) set how fast a pressed look arrives and leaves, which [[Base Module]] uses. `BOUNCY` is also the selected row bar's spring in [[Interaction Module]]. `NOTE_HOLD`, `LEVEL_HOLD` and `TOAST_HOLD` are times, not motions: how long "Saved", the zoom level and a toast stay.

## Items

- [[Spring]] — A spring as Framer Motion describes one.
- `pub const SNAPPY: Spring` — `spring.snappy`: the default for things that enter or follow the pointer.
- `pub const BOUNCY: Spring` — `spring.bouncy`: overshoots; for dialogs and small accents.
- `pub const PAGE: Spring` — the spring of `pageEntrance` and `scrollReveal`.
- `pub const PAGE_RISE: f32` — travel of `pageEntrance`, in pixels.
- `pub const CARD_RISE: f32` — travel of a `staggerCascade` item.
- `pub const CARD_STAGGER: Duration` — `staggerCascade`: time between two cards.
- `pub const STAGGER_CAP: usize` — items after this many enter together with the last staggered one. Without a cap the last row of a long table would arrive seconds late.
- `pub const EXIT: Duration` — `scaleIn` exit, and the exit of `tabSwitch` and `pageEntrance`.
- `pub const NOTE_HOLD: Duration` — how long the word "Saved" stays beside a setting. Not a motion of MUI or of TradingAgent: long enough to be read, and gone before the next change.
- `pub const LEVEL_HOLD: Duration` — how long the zoom level stays in the title bar after a change. Not a motion of MUI or of TradingAgent: long enough to read the figure while the wheel turns, and gone soon after it stops.
- `pub const TOAST_HOLD: Duration` — how long a toast stays on screen. Not a motion: the `autoHideDuration` of MUI's Snackbar docs example (`autoHideDuration={6000}`), long enough to read a title and a line under it.
- `pub fn scale() -> f32` — the factor every motion is stretched by; 1 unless `CODEWIKI_MOTION_SCALE` says otherwise.
- `pub fn scaled(duration: Duration) -> Duration` — `duration` stretched by `scale`.
- `drawer` — how much of a side pane shows, from 0 (hidden) to 1 (shown), on the way to `open`: MUI's persistent `Drawer`. It enters over `enteringScreen` on `easeOut` and leaves over `leavingScreen` on `sharp` (`Drawer/Drawer.js`, `Slide/Slide.js`). A pane that starts hidden starts at 0, with no motion.
- `pub fn glide(elapsed: Duration) -> f32` — how far a glide, such as the note's scroll to a heading the outline names, has gone after `elapsed`: from 0 to exactly 1 over MUI's `standard` duration, eased in and out.
- `pub fn count_up(value: &str, progress: f32) -> String` — A figure such as `286`, `62.5%` or `1,204 lines`, `progress` of the way up from zero: TradingAgent's `RollingDigits`. Text that does not start with a number is returned as it is.
- `pub fn stagger_delay(index: usize, step: Duration) -> Duration` — how long item `index` of a staggered group waits before it enters.
- `pub struct Enter` — an entrance: where the element comes from, on which spring, after what wait.
- `pub trait EnterExt: Styled + IntoElement + Sized + 'static` — elements that can make an entrance.

And 7 more items.

## Depends on

- [[Spring]] — uses `Spring`, `animation`, `mass`, `new`
- [[Curve Module]] — uses `EASE_IN_OUT`, `EASE_OUT`, `SHARP`
- [[Duration Module]] — uses `ENTERING_SCREEN`, `LEAVING_SCREEN`, `STANDARD`
- [[Tokens Module]] — uses `easing`
- [[Zoom Module]] — uses `px`

## Used by

- [[codewiki-app]] — from `check_for_update`, `connect`, `context_menu_view`, `diagram_viewer_view`
- [[Doc View Module]] — from `column`, `draw_strip`, `render`, `stat_tile`
- [[Ripple Module]] — from `finished`, `frame`, `moving`
- [[Spring]] — from `animation`, `config`, `policy`
- [[Button Module]] — from `ease`, `render`
- [[Autocomplete Module]] — from `render`
- [[Base Module]] — from `new`
- [[Checkbox Module]] — from `render`

## Gotchas

- `CODEWIKI_MOTION_SCALE` is read once at first use. Changing it later in the same run has no effect.
- A value below 1 is ignored, not honored.
- A spring may overshoot past 1. The entrance offset follows it but opacity does not, and `count_up` clamps so a figure never exceeds its target.
- `glide` does not overshoot: it is eased, not a spring, and it never goes past 1.
- `glide` ignores the reduced motion setting. The caller checks it and skips the glide.
- Under reduced motion an entrance is simply there.
- `count_up` returns text that does not start with a digit unchanged.
- `NOTE_HOLD` and `LEVEL_HOLD` are the app's own, not from MUI or TradingAgent. `TOAST_HOLD` is the 6 seconds of MUI's Snackbar docs example.

## See also

- [[_index]]
- [[codewiki-app]]

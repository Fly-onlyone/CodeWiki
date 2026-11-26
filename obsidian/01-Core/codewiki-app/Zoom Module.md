---
tags: [core, codewiki-app, module]
---

# Zoom Module

> Zoom: the whole app larger or smaller, as a browser zooms a page.

## What it does

Ctrl+wheel, Ctrl+= and Ctrl+- step through `LEVELS`; Ctrl+0 goes back to 100%. The level is remembered between runs.

GPUI cannot change a window's scale: the display decides it. What GPUI offers is the rem, the size its `text_sm`, `p_2` and the like are measured in, which gpui-kit's root sets from the theme's font size on every frame. Most of the app is measured in pixels, though. So zoom works in two places:

- The app's own sizes. A size written as an `f32` is in design units, and `px` is the one way to turn it into GPUI's pixels: it multiplies by the zoom. The app's modules use this `px` in place of GPUI's. A pixel value turned back into a number (`width / px(1.)`) is then in design units too, so measuring and drawing agree at every level.
- The theme. `sync_theme` scales its font sizes and radii, which moves the rem and with it every gpui-kit control.

The title bar stays as it is, as a browser's does: it is built at 100% (`unscaled`) and laid out in the unzoomed rem (`FixedRem`).

## Source

- `crates/codewiki-app/src/zoom.rs` — module `codewiki_app::zoom`

## How it works

1. `AppView::body` and `AppView::details` in [[codewiki-app]], and 71 more, call `px`. `px` calls `factor`.
2. `title_bar` in [[codewiki-app]] and `zoom_card` in [[Settings Pages Module]] call `label`.
3. `AppView::run_command` and `main` in [[codewiki-app]], and 1 more, call `step_by`. `step_by` calls `set`, then `step`, then `percent`.
4. `DocPage::new` and `DocPage::render` in [[Doc View Module]] call `factor`.
5. `Restore::drop` in [[codewiki-app]] calls `set_factor`.

11 more entry points: `unscaled`, `notches`, `init`, `percent`, `set`, `reset`, and 5 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const LEVELS: [u16; 11]` — the levels, in percent: Chrome's zoom levels from 50% to 200%.
- `pub const DEFAULT: u16` — the level the app starts at until the user picks another.
- `pub fn factor() -> f32` — the zoom as a factor, 1 at 100%.
- `pub fn px(n: f32) -> Pixels` — `n` design units in GPUI's pixels, at the current zoom.
- `pub fn unscaled<R>(build: impl FnOnce() -> R) -> R` — run `build` at 100%: what it measures with `px` is not zoomed. The zoom comes back afterwards, even if `build` panics.
- `pub fn label(percent: u16) -> String` — how a level reads: `125%`.
- `pub fn step(percent: u16, steps: i32) -> u16` — the level `steps` steps from `percent`, up for a positive count. A value between two levels counts as already part of the way to the next one, so its first step lands on that one. The ends hold.
- `pub fn notches(delta: ScrollDelta, lines_per_notch: u32) -> f32` — how many wheel notches `delta` is, up for positive. Windows reports a notch as `lines_per_notch` lines (the "lines to scroll" of its mouse settings).
- `pub struct Wheel` — turns wheel notches into whole steps. A high-resolution wheel or a touchpad sends parts of a notch; they add up.
- `pub struct Indicator` — the chip in the title bar that shows the level after a change.
- `pub fn init(cx: &mut App)` — apply the remembered level. Call after `themes::init`, so that the theme's sizes are still at 100% when they are taken as the base.
- `pub fn percent(cx: &App) -> u16` — the level in effect, in percent.
- `pub fn set(percent: u16, cx: &mut App)` — go to `percent`, apply it, remember it, and show it in the title bar.
- `pub fn step_by(steps: i32, cx: &mut App)` — go `steps` levels up, or down for a negative count.
- `pub fn reset(cx: &mut App)` — go back to 100%.
- `pub fn indicator_shown(cx: &App) -> bool` — whether the level chip is shown.
- `pub fn sync_theme(cx: &mut App) -> bool` — make the theme's sizes those of the current level. Cheap when they are already, so it runs on every frame: gpui-kit writes the theme file's sizes back into the theme when its list of themes changes, and that can happen after zoom has scaled them. Returns whether it changed anything.
- `pub fn rebase_theme(cx: &mut App)` — take the theme's sizes as they are now as its sizes at 100%, and scale them. Call after a theme was applied: its sizes are then unzoomed.
- `pub fn base_rem(cx: &App) -> Pixels` — the rem at 100%: the theme's font size before zoom.
- `pub struct FixedRem` — lays out and draws `child` with the rem at `rem`, whatever the window's is. GPUI resolves rems while it lays out and draws, not when an element is built, so a size in rems follows the rem in effect at that time.

And 12 more items.

## Depends on

- [[Motion Module (mui)]] — uses `LEVEL_HOLD`, `scaled`
- [[Prefs Module]] — uses `read`, `write`

## Used by

- [[codewiki-app]] — from `body`, `details`, `diagram_viewer_view`, `drop`
- [[Doc View Module]] — from `caption`, `choices_row`, `code_block`, `column`
- [[Feedback Module]] — from `alert_action`, `alert_titled`, `chip`, `dots`
- [[List Module]] — from `bar_glow`, `icon_glow`, `render`, `subheader`
- [[Surfaces Module]] — from `card_content`, `card_shadow`, `divider`, `text`
- [[Backdrop Module]] — from `Shell`, `new`, `render`
- [[Glass Module]] — from `hairline`, `reading_surface`
- [[Illustration Module]] — from `flow`, `step`

## See also

- [[_index]]
- [[codewiki-app]]

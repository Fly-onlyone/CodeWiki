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

The zoom factor lives in a thread-local, set by `set_factor`. `px(n)` multiplies a design size by it. `init` reads the saved level through [[Prefs Module]], falling back to `DEFAULT`; `parse` accepts only values listed in `LEVELS`.

`set` snaps the level to `LEVELS`, stores it as a global, updates the factor, calls `sync_theme`, saves it, starts the title bar chip, and calls `refresh_windows` so cached views redraw. `step_by` and `reset` go through `set`. `step` finds the neighbour level; a value between levels moves to the nearer one in the direction asked, and the ends hold.

Wheel input: `notches` converts a scroll delta into notches, using the Windows lines-per-notch setting for line deltas and 50 pixels per notch for touchpads. `Wheel::feed` adds parts of notches and returns whole steps. A pause over 400 ms or a turn the other way resets the sum.

Theme: `Sizes` holds the font, mono font and two radii. `rebased` keeps a base at 100%. If the theme still holds what zoom last wrote, the base stays. If something else changed the theme, its sizes become the new base. `sync_theme` runs every frame and writes only when needed.

`show_indicator` sets a sequence number, and a timer hides the chip only if no later change came.

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

- [[codewiki-app]] — from `body`, `context_menu_view`, `details`, `diagram_viewer_view`
- [[Doc View Module]] — from `caption`, `choices_row`, `code_block`, `column`
- [[Feedback Module]] — from `alert_action`, `alert_titled`, `chip`, `dots`
- [[List Module]] — from `bar_glow`, `icon_glow`, `render`, `subheader`
- [[Surfaces Module]] — from `card_content`, `card_shadow`, `divider`, `text`
- [[Backdrop Module]] — from `Shell`, `new`, `render`
- [[Glass Module]] — from `hairline`, `reading_surface`
- [[Illustration Module]] — from `flow`, `step`

## Gotchas

- The factor is thread-local: only the main thread draws, and each test sees 1.0 on its own thread.
- Use this module's `px`, not GPUI's, for app sizes. GPUI's `px` is allowed only here (`clippy::disallowed_methods` is lifted for this file).
- Call `init` after `themes::init`, so the theme is still at 100% when taken as the base. After applying a theme, call `rebase_theme`.
- `sync_theme` must run per frame: gpui-kit can write the theme file's sizes back after zoom scaled them.
- `unscaled` restores the factor even when the builder panics (a drop guard does it).
- A wheel set to one screen per notch reports `u32::MAX` lines; `notches` still gives 1. Scrolling turned off gives 0 and zooms nothing.
- The title bar is built unscaled and laid out in `FixedRem`. Rems resolve at layout and paint time, not when the element is built, so the fixed rem must wrap layout and paint.

## See also

- [[_index]]
- [[codewiki-app]]

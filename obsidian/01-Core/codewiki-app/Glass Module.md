---
tags: [core, codewiki-app, module]
---

# Glass Module

> Glass: how see-through each surface is.

## What it does

The app window sits on the Windows Mica backdrop (the blurred wallpaper), and every surface in it is the theme's color at less than full opacity. This module decides how see-through each surface is. Glass is the look this gives.

There are three kinds of surface. The pane carries the page text and must stay readable. The chrome is the frame around it: title bar, sidebar, toolbar, tabs and details. The margin is the empty side of a page beside its reading column. It holds no text, so it can be the clearest.

The levels are not fixed. Each surface is made as clear as it can be while its text keeps the contrast that [[Contrast Module]] asks for, so a theme with weak text gets denser glass. Surfaces and borders are also tinted a little toward the theme's primary color, to give every theme a mood.

## Source

- `crates/codewiki-app/src/glass.rs` — module `codewiki_app::glass`

## How it works

Once per frame, `Levels::for_window` reads the active theme and whether Windows is in dark mode. It tints the background toward the primary color for the pane and the chrome, and passes both to `Levels::compute` with the three text colors (foreground, muted, link).

`Levels::compute` asks `contrast::fit_alpha` for the lowest opacity that keeps every text readable. The pane is checked against any backdrop, black or white. The chrome depends on the match between theme and system mode. When both are dark or both light, the backdrop is known to be near the theme's side, so `matched_backdrop` gives a narrow range and a low floor (0.62) is enough. When they differ, the floor is 0.86 against any backdrop. The chrome is then capped at the pane's opacity. The margin takes 0.35 on a match, else the chrome's value.

`reading_surface` lays out the result: a margin strip, a fade, the column, a fade, a margin strip. `margin_width` gives the strip width from the page gutter, or `None` when there is no room.

## Items

- `pub(crate) const CHROME_MATCHED_FLOOR: f32` — chrome opacity when the backdrop is known to be on the theme's side: a dark theme on a dark system, or light on light.
- `pub(crate) const MARGIN_MATCHED: f32` — margin opacity when the backdrop is on the theme's side. A margin holds no text, so this is a matter of taste, not of contrast; chosen by eye on 2026-11-24. On the other side the margin takes the chrome's opacity: a dark page on clear light glass looks like a hole.
- `pub(crate) const FADE: f32` — width of the fade from a margin to the reading column, in design units.
- `const COLUMN_INSET: f32` — space at the column's full opacity on each side of its text, so that the fade never runs under a letter, in design units.
- `const MIN_MARGIN: f32` — the narrowest a margin may be, fade not counted, in design units. A narrower one is left at the column's opacity.
- `pub(crate) const CHROME_TINT: f32` — share of the primary color mixed into every chrome surface.
- `pub const PANE_TINT: f32` — share of the primary color mixed into the reading pane, as much as the chrome so that the page and its frame read as one surface.
- `const BORDER_PULL: f32` — how far borders are pulled toward the primary color.
- `const DARK_MICA_LIMIT: Rgb` — mica is the wallpaper, blurred and pulled toward the system mode's base color. These are the lightest dark Mica and the darkest light Mica.
- `pub(crate) fn matched_backdrop(system_dark: bool) -> [Rgb; 2]` — what can be behind the chrome when the system is in the theme's mode: Mica, with the drifting backdrop over it. The drifting shapes are theme colors at `backdrop::STRENGTH`; taking them as pure white on a dark system and pure black on a light one bounds any color they can be.
- `pub struct Levels` — opacity of the two kinds of surface, from 0 (clear) to 1 (solid).
- `fn rgb(color: Hsla) -> Rgb` — private function, returns `Rgb`; called by `Levels::for_window`.
- `pub fn tint(surface: Hsla, primary: Hsla, share: f32) -> Hsla` — `surface` with `share` of `primary` mixed in.
- `pub fn tinted(surface: Hsla, cx: &App) -> Hsla` — A chrome surface: `surface` tinted toward the active theme's primary color. Pass the result to `Levels::chrome`.
- `pub fn tinted_pane(surface: Hsla, cx: &App) -> Hsla` — the reading pane's surface: `surface` tinted toward the active theme's primary color. Pass the result to `Levels::pane`.
- `pub fn margin_width(gutter: f32) -> Option<f32>` — how wide each clear margin is beside a reading column whose text starts `gutter` in from the pane's sides, fade not counted; `None` when there is no room for one. In design units.
- `pub fn reading_surface(levels: Levels, margin: f32, cx: &App) -> Div` — the surface of a reading pane: clear margins `margin` wide (from `margin_width`) on each side, soft fades, and the column at the pane's opacity between them. Lay it under the page, filling the pane.
- `pub fn tinted_border(cx: &App) -> Hsla` — the theme's border, pulled toward its primary color.
- `pub fn hairline(cx: &App) -> Div` — A one-pixel strip across the top of a surface that starts in the primary color and fades out to the right. One pixel of the screen at every zoom, as a border is.
- `pub fn debug_backdrop() -> Option<Hsla>` — an opaque color to paint under every surface instead of the real backdrop, from `CODEWIKI_BACKDROP=#RRGGBB`. A window capture cannot see Mica, so this is how the glass is checked: with a known color behind it, a capture shows exactly what each surface lets through.

And 3 more items.

## Depends on

- [[Contrast Module]] — uses `BLACK`, `PANE_FLOOR`, `Rgb`, `WHITE`
- [[Backdrop Module]] — uses `STRENGTH`
- [[Zoom Module]] — uses `px`

## Used by

- [[codewiki-app]] — from `AppView`, `body`, `details`, `margin_width`
- [[Surfaces Module]] — from `card`, `card_fill`, `card_piece`, `paper`
- [[Backdrop Module]] — from `new`
- [[Reading Module]] — from `style`
- [[Standard Module]] — from `nudge_texts`

## Gotchas

- Chrome is never denser than the pane, by design. A test checks it for both modes.
- When the system mode and the theme mode differ, the margin copies the chrome. The source says a dark page on clear light glass looks like a hole.
- A window capture cannot see Mica. Set `CODEWIKI_BACKDROP=#RRGGBB` and `debug_backdrop` returns a solid color to paint under everything, so a capture shows what each surface lets through.
- Margin and fade sizes are design units. Draw them through the zoom `px`, not raw pixels. `hairline` is the exception: it stays one screen pixel at every zoom.
- A test requires every bundled theme to reach the clearest glass on a matching system. A new theme with weak text would get denser glass and look like another app.

## See also

- [[_index]]
- [[codewiki-app]]

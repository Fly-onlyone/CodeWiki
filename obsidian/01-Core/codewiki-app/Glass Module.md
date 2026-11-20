---
tags: [core, codewiki-app, module]
---

# Glass Module

> Glass: how see-through each surface is.

## What it does

The window sits on the Windows Mica backdrop and every surface is the theme's color at less than full opacity. gpui-kit cannot blur inside a window, so the backdrop is the only source of the glass look.

Two kinds of surface exist. The *pane* carries the page: long text that must stay readable whatever is behind the window. The *chrome* is everything around it — title bar, sidebar, toolbar, tabs, details — and shows more of the backdrop.

How opaque each must be is worked out from the theme, not fixed: a surface is as clear as it can be while its text keeps the contrast `contrast` asks for. A theme with weak contrast therefore gets denser glass.

The chrome, the page and the borders are all tinted a little toward the theme's primary color, so that every theme has a mood of its own. The tint is faint: the theme's three brand colors carry its hues, and surfaces that are as colored as they are would drown them.

## Source

- `crates/codewiki-app/src/glass.rs` — module `codewiki_app::glass`

## How it works

1. `AppView::details` and `AppView::sidebar` in [[codewiki-app]], and 7 more, call `tinted_border`.
2. `AppView::body` and `AppView::palette_view` in [[codewiki-app]], and 4 more, call `hairline`.
3. `nudge_texts` in [[Standard Module]] calls `matched_backdrop`. `matched_backdrop` calls `over` in [[Contrast Module]].
4. `AppView::render` and `title_bar` in [[codewiki-app]] call `Levels::for_window`. `Levels::for_window` calls `Levels::compute`, then `rgb`, then `tint`. `Levels::compute` calls `fit_alpha` in [[Contrast Module]], then `matched_backdrop`.
5. `style` in [[Reading Module]] calls `tint`.

3 more entry points: `tinted`, `tinted_pane`, `debug_backdrop`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub(crate) const CHROME_MATCHED_FLOOR: f32` — chrome opacity when the backdrop is known to be on the theme's side: a dark theme on a dark system, or light on light.
- `const CHROME_FLOOR: f32` — chrome opacity when it is not: a dark theme over a light backdrop needs far denser glass to keep muted text readable.
- `pub(crate) const CHROME_TINT: f32` — share of the primary color mixed into every chrome surface.
- `pub const PANE_TINT: f32` — share of the primary color mixed into the reading pane, as much as the chrome so that the page and its frame read as one surface.
- `const BORDER_PULL: f32` — how far borders are pulled toward the primary color.
- `const DARK_MICA_LIMIT: Rgb` — mica is the wallpaper, blurred and pulled toward the system mode's base color. These are the lightest dark Mica and the darkest light Mica.
- `const UNKNOWN: [Rgb; 2]` — any backdrop at all.
- `pub(crate) fn matched_backdrop(system_dark: bool) -> [Rgb; 2]` — what can be behind the chrome when the system is in the theme's mode: Mica, with the drifting backdrop over it. The drifting shapes are theme colors at `backdrop::STRENGTH`; taking them as pure white on a dark system and pure black on a light one bounds any color they can be.
- `pub struct Levels` — opacity of the two kinds of surface, from 0 (clear) to 1 (solid).
- `fn rgb(color: Hsla) -> Rgb` — private function, returns `Rgb`; called by `Levels::for_window`.
- `pub fn tint(surface: Hsla, primary: Hsla, share: f32) -> Hsla` — `surface` with `share` of `primary` mixed in.
- `pub fn tinted(surface: Hsla, cx: &App) -> Hsla` — A chrome surface: `surface` tinted toward the active theme's primary color. Pass the result to `Levels::chrome`.
- `pub fn tinted_pane(surface: Hsla, cx: &App) -> Hsla` — the reading pane's surface: `surface` tinted toward the active theme's primary color. Pass the result to `Levels::pane`.
- `pub fn tinted_border(cx: &App) -> Hsla` — the theme's border, pulled toward its primary color.
- `pub fn hairline(cx: &App) -> Div` — A one-pixel strip across the top of a surface that starts in the primary color and fades out to the right. One pixel of the screen at every zoom, as a border is.
- `pub fn debug_backdrop() -> Option<Hsla>` — an opaque color to paint under every surface instead of the real backdrop, from `CODEWIKI_BACKDROP=#RRGGBB`. A window capture cannot see Mica, so this is how the glass is checked: with a known color behind it, a capture shows exactly what each surface lets through.

And 1 more private item with no comment.

## Depends on

- [[Contrast Module]] — uses `BLACK`, `PANE_FLOOR`, `Rgb`, `WHITE`
- [[Backdrop Module]] — uses `STRENGTH`

## Used by

- [[codewiki-app]] — from `AppView`, `body`, `details`, `new`
- [[Surfaces Module]] — from `card`, `card_fill`, `card_piece`, `paper`
- [[Backdrop Module]] — from `new`
- [[Reading Module]] — from `style`
- [[Standard Module]] — from `nudge_texts`

## See also

- [[_index]]
- [[codewiki-app]]

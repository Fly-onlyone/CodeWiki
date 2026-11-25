---
tags: [core, codewiki-app, module]
---

# Surfaces Module

> `Paper`, `Card` and `Divider`: what content sits on, and `Typography`, how it is set.

## What it does

The card is TradingAgent's (`components.ts`, `MuiCard`): tinted glass with a thin primary border, corners four pixels rounder than a button's, a soft shadow and a line of accent color along its top edge. TradingAgent blurs what is behind a card; GPUI cannot, so the card is kept dense enough to read on whatever is behind it.

## Source

- `crates/codewiki-app/src/mui/surfaces.rs` — module `codewiki_app::mui::surfaces`

## How it works

1. `AppView::diagram_viewer_view` and `AppView::palette_view` in [[codewiki-app]], and 2 more, call `paper`. `paper` calls `tinted_border` in [[Glass Module]], then `elevation` in [[Tokens Module]].
2. `AppView::details` and `AppView::diagram_viewer_view` in [[codewiki-app]], and 14 more, call `text`. `text` calls `px` in [[Zoom Module]].
3. `code_block` and `draw` in [[Doc View Module]], and 4 more, call `card`. `card` calls `tinted_border` in [[Glass Module]], then `card_fill`, then `card_shadow`, then `hairline` in [[Glass Module]]. `card_shadow` calls `px` in [[Zoom Module]].
4. `draw_strip` in [[Doc View Module]] calls `card_piece`. `card_piece` calls `tinted_border` in [[Glass Module]], then `card_fill`, then `hairline` in [[Glass Module]].
5. `draw` and `empty` in [[Doc View Module]], and 1 more, call `card_content`. `card_content` calls `px` in [[Zoom Module]].

1 more entry point: `divider`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const CARD_PADDING: f32` — MUI's `CardContent`: `padding: 16`.
- `const CARD_TINT: f32` — `MuiCard`: the share of primary color mixed into a card.
- `fn card_shadow() -> Vec<BoxShadow>` — `MuiCard`, `boxShadow`: `0 4px 20px black/.2, inset 0 1px 0 white/.05`.
- `pub fn card_fill(cx: &App) -> Hsla` — the fill of a card: the theme's surface color, as dense as the reading pane, so text on it keeps its contrast.
- `pub fn card(cx: &App) -> Div` — A card: a bordered, tinted surface for one group of content. Its children are laid out in a column with MUI's `CardContent` padding.
- `pub enum Piece` — which part of a card a piece is, when a card is drawn in pieces.
- `pub fn card_piece(piece: Piece, cx: &App) -> Div` — one piece of a card that is drawn as several elements one under another, such as a long table whose rows a list builds one at a time. Stacked, the pieces look like a `card`, without its shadow, which would show a seam at every joint.
- `pub fn card_content() -> Div` — the padded body of a `card`.
- `pub fn paper(elevation: usize, cx: &App) -> Div` — A raised surface for something that floats over the page, such as a dialog: opaque, with MUI's elevation shadow.
- `pub fn divider(cx: &App) -> Div` — A one pixel line between two things.
- `pub fn text(variant: Variant, content: impl Into<SharedString>) -> Div` — text in one of MUI's styles.

## Depends on

- [[Glass Module]] — uses `Levels`, `OPAQUE`, `hairline`, `pane`
- [[Tokens Module]] — uses `SPACING`, `Variant`, `elevation`, `spec`
- [[Zoom Module]] — uses `px`

## Used by

- [[Doc View Module]] — from `caption`, `code_block`, `column`, `draw`
- [[codewiki-app]] — from `details`, `diagram_viewer_view`, `palette_view`, `settings_frame`
- [[Illustration Module]] — from `flow`, `render`, `step`
- [[Autocomplete Module]] — from `render`
- [[Menu Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]

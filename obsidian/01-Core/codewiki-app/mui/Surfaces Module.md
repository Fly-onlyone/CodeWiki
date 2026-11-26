---
tags: [core, codewiki-app, module]
---

# Surfaces Module

> `Paper`, `Card` and `Divider`: what content sits on, and `Typography`, how it is set.

## What it does

This module holds the surfaces the app's content sits on, and the one function that sets text. A `card` groups content on the page. A `paper` is a raised surface for things that float, such as a menu, a dialog or a toast. A `divider` is a 1 px line. `text` sets a string in one of MUI's typography styles.

The card follows TradingAgent's look: tinted glass with a thin primary border, rounder corners than a button, a soft shadow and a hairline of accent color along the top. TradingAgent blurs what is behind a card. GPUI cannot blur, so the card fill is dense enough to read on any backdrop.

## Source

- `crates/codewiki-app/src/mui/surfaces.rs` — module `codewiki_app::mui::surfaces`

## How it works

`card` is a column with the large radius, a tinted border from the [[Glass Module]], and a fill from `card_fill`. The fill is the theme's secondary color run through the glass `pane` level, which makes it as dense as the reading pane so text keeps its contrast. A faint primary tint lies over the fill, and a hairline runs along the top inside the corners. `card_content` is the padded column to put inside (16 px padding).

`card_piece` is for a card drawn as several stacked elements, such as a long table built row by row. `Piece::Top` has the top border, the rounded top and the hairline. `Middle` has side borders only. `Bottom` has the bottom border and the rounded bottom. Stacked, they read as one card.

`paper` is opaque, uses the popover colors, and takes its shadow from `tokens::elevation`. `text` reads the size, line height, weight and uppercase flag from the variant spec, all through the zoom module's `px`.

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
- [[codewiki-app]] — from `details`, `diagram_viewer_view`, `palette_view`, `review_details`
- [[Illustration Module]] — from `flow`, `render`, `step`
- [[Autocomplete Module]] — from `render`
- [[Menu Module]] — from `render`
- [[Snackbar Module]] — from `snackbar`

## Gotchas

- `card_piece` has no shadow on purpose. A shadow on each piece would show a seam at every joint.
- The hairline in `card` and the top piece is inset by the corner radius. GPUI clips to rectangles, so a full-width line would stick out past the rounded corners.
- Without a glass levels global, `card_fill` falls back to opaque.
- `paper` is made for dialogs. A toast sets its own corner radius on top of it.
- `text` upper-cases the string itself when the variant asks for it, so the original casing is lost in the element.
- The shadow is drawn from `card_shadow` on every call; it is not cached.

## See also

- [[_index]]
- [[codewiki-app]]

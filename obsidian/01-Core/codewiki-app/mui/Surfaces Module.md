---
tags: [core, codewiki-app, module]
---

# Surfaces Module

> `Paper`, `Card` and `Divider`: what content sits on, and `Typography`, how it is set.

## Source

- `crates/codewiki-app/src/mui/surfaces.rs` — module `codewiki_app::mui::surfaces`

## How it works

Main items:

- `text` — text in one of MUI's styles.
- `card` — A card: a bordered, tinted surface for one group of content.
- `CARD_PADDING` — MUI's `CardContent`: `padding: 16`.
- `card_fill` — the fill of a card: the theme's surface color, as dense as the reading pane, so text on it keeps its contrast.
- `card_content` — the padded body of a `card`.
- `paper` — A raised surface for something that floats over the page, such as a dialog: opaque, with MUI's elevation shadow.
- `Piece` — which part of a card a piece is, when a card is drawn in pieces.
- `card_piece` — one piece of a card that is drawn as several elements one under another, such as a long table whose rows a list builds one at a time.

## Depends on

- [[Glass Module]] — uses `Levels`, `OPAQUE`, `hairline`, `pane`
- [[Tokens Module]] — uses `SPACING`, `Variant`, `elevation`, `spec`

## Used by

- [[Doc View Module]] — from `caption`, `code_block`, `column`, `draw`
- [[codewiki-app]] — from `details`, `palette_view`, `settings_frame`
- [[Illustration Module]] — from `render`, `steps`
- [[Menu Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]

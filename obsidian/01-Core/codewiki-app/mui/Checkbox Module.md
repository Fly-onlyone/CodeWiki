---
tags: [core, codewiki-app, module]
---

# Checkbox Module

> `Checkbox`, at MUI's small size.

## What it does

Sizes and colors are MUI's (`Checkbox/Checkbox.js`, `internal/SwitchBase.js`, `SvgIcon/SvgIcon.js`); TradingAgent does not restyle the checkbox. A round area around one of three icons: a blank box, a checked box, and the box of a selection that is partly made. The area tints under the pointer and carries the ripple.

MUI swaps the icon at once and does not ease the tint; here the tint eases over `duration.shortest`, as the app's other controls do.

## Source

- `crates/codewiki-app/src/mui/checkbox.rs` — module `codewiki_app::mui::checkbox`

## How it works

1. `AppView::review_queue` in [[codewiki-app]] calls `Check::of`.
2. `AppView::review_queue` in [[codewiki-app]] calls `Checkbox::new`.

## Items

- `const ICON: f32` — `SvgIcon.js`, `fontSize: 'small'`: `pxToRem(20)`. `Checkbox.js` gives its icons the checkbox's own size.
- `const PADDING: f32` — `SwitchBase.js`, `SwitchBaseRoot`: `padding: 9`, `borderRadius: '50%'`.
- `pub const SIZE: f32` — the checkbox with the round area around its icon.
- `const EDGE_START: f32` — `SwitchBase.js`: `edge: 'start'` at `size: 'small'` is `marginLeft: -3`, so that the box lines up with the text above it.
- `pub enum Check` — what a checkbox shows.
- `pub struct Look` — the colors of a checkbox in one state.
- `pub struct Checkbox` — A box that is checked, unchecked or partly checked, and says when it is clicked. What a click changes is the owner's to decide.

## Depends on

- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Interaction Module]] — uses `DARK`, `for_mode`
- [[Tokens Module]] — uses `easing`, `states`
- [[Motion Module (mui)]] — uses `scaled`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `SHORTEST`
- [[Zoom Module]] — uses `px`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `review_queue`
- [[Review Module]] — from `header`

## See also

- [[_index]]
- [[codewiki-app]]

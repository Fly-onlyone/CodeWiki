---
tags: [core, codewiki-app, module]
---

# Button Module

> `Button` and `IconButton`.

## Source

- `crates/codewiki-app/src/mui/button.rs` — module `codewiki_app::mui::button`

## How it works

Main items:

- [[Button]] — A button with a label, and optionally an icon before it.
- `IconButton` — A round button that is only an icon.
- `ToggleButtonGroup` — A few buttons side by side, one of them chosen: MUI's exclusive `ToggleButtonGroup` in the primary color.
- `Metrics` — what a size comes to for a button of one kind.
- `Scale` — MUI's `size`.
- `Tone` — MUI's `color`.
- `Kind` — MUI's `variant`.
- `icon_button_metrics` — `IconButton.js`: padding around the icon, and the icon's size.

## Depends on

- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Tokens Module]] — uses `easing`, `spec`, `states`
- [[Doc View Module]] — uses `id`
- [[Feedback Module]] — uses `tooltip`
- [[Motion Module (mui)]] — uses `scaled`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `SHORTEST`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `tab_bar`, `toolbar`
- [[Button]] — from `Button`, `render`
- [[Doc View Module]] — from `choices_row`

## See also

- [[_index]]
- [[codewiki-app]]

---
tags: [core, codewiki-app, module]
---

# Text Field Module

> `TextField`, in MUI's outlined variant and small size.

## What it does

The editing itself is gpui-kit's `Input`, with its own border and fill turned off. This draws what MUI draws around an input: the outline, which takes the primary color and a second pixel when the field has focus, an icon before the text, and a line of help or of news under it (`OutlinedInput/OutlinedInput.js`, `InputAdornment/InputAdornment.js`, `FormHelperText/FormHelperText.js`). The outline's colors at rest and under the pointer, and the fill of a disabled field, are TradingAgent's (`components.ts`, `MuiTextField` and `MuiInputBase`).

A field that hides what is typed is the same field over an input that is masked (`InputState::masked`).

## Source

- `crates/codewiki-app/src/mui/text_field.rs` — module `codewiki_app::mui::text_field`

## How it works

1. `AppView::palette_view` and `AppView::toolbar` in [[codewiki-app]], and 2 more, call `TextField::new`.

## Items

- `pub const HEIGHT: f32` — `OutlinedInput.js`, `size: 'small'`: `padding: '8.5px 14px'` around a line of `1.4375em` at 16 px.
- `const ICON_GAP: f32` — `InputAdornment.js`: `marginRight: 8`.
- `const OUTLINE_OPACITY: f32` — `OutlinedInput.js`: the outline is the text color at 23%.
- `const DISABLED_FILL: f32` — `MuiInputBase`, disabled: the sidebar color at 50% behind the field.
- `const HELPER_TOP: f32` — `FormHelperText.js`, `size: 'small'`: `marginTop: 4`; contained: `marginLeft: 14`, `marginRight: 14`.
- `pub struct TextField` — A single-line text field.

And 4 more private items with no comment.

## Depends on

- [[Tokens Module]] — uses `easing`, `spec`, `states`
- [[Base Module]] — uses `new`
- [[Motion Module (mui)]] — uses `scaled`
- [[Curve Module]] — uses `EASE_OUT`
- [[Duration Module]] — uses `SHORTER`
- [[Zoom Module]] — uses `px`
- [[Glyph]] — uses `Glyph`
- [[Tone]] — uses `Tone`

## Used by

- [[Doc View Module]] — from `choices_row`, `draw`, `row`, `secret_control`
- [[codewiki-app]] — from `palette_view`, `toolbar`
- [[Select Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]

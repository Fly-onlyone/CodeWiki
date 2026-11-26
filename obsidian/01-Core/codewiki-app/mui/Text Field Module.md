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

The caller makes an `InputState` entity that holds the text and the cursor, and passes it to `TextField::new` with an id. An optional start icon, a disabled flag and a helper line (text and `Tone`) are added by builder calls.

`render` finds out whether the field has focus, the pointer is over it, and whether the helper is an error. It then picks the outline color. Disabled uses the foreground at the disabled opacity. Focus or an error uses the accent color: primary, or the error color for an error. A hover uses primary at the app's outline-hover opacity. Rest is the foreground at 23%. The color eases over `duration::SHORTER` with the ease-out curve.

MUI widens the outline to two pixels on focus. Here the border stays 1 px and a second pixel is drawn as an inner shadow whose color fades in. The text does not move.

The field is 40 px high, with 14 px side padding, a 20 px icon and 8 px gap. Inside it, gpui-kit's `Input` has its own appearance turned off. A disabled field gets the sidebar color at 50% as fill and no hover tracking. The helper is a caption line below, truncated, in the tone's color.

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
- [[Interaction Module]] — uses `current`
- [[Motion Module (mui)]] — uses `scaled`
- [[Curve Module]] — uses `EASE_OUT`
- [[Duration Module]] — uses `SHORTER`
- [[Zoom Module]] — uses `px`
- [[Glyph]] — uses `Glyph`

## Used by

- [[Doc View Module]] — from `choices_row`, `draw`, `row`, `secret_control`
- [[codewiki-app]] — from `palette_view`, `toolbar`
- [[Select Module]] — from `render`

## Gotchas

- An error helper colors the outline even when the field is not focused. It overrides hover and rest, though not disabled.
- The field keeps no text. Read and write the value through the `InputState` entity.
- A password field is the same widget with a masked `InputState`.
- Focus is read from the input's focus handle, so a disabled field never counts as focused.
- `HEIGHT` is public because a select uses it to line up in a row with a text field.
- The helper line is truncated to one line. A long error message is cut off.

## See also

- [[_index]]
- [[codewiki-app]]

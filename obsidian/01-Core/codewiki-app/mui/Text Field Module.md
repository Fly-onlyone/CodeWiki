---
tags: [core, codewiki-app, module]
---

# Text Field Module

> `TextField`, in MUI's outlined variant and small size.

## Source

- `crates/codewiki-app/src/mui/text_field.rs` — module `codewiki_app::mui::text_field`

## How it works

Main items:

- `TextField` — A single-line text field.
- `HEIGHT` — `OutlinedInput.js`, `size: 'small'`: `padding: '8.5px 14px'` around a line of `1.4375em` at 16 px.
- `DISABLED_FILL` — `MuiInputBase`, disabled: the sidebar color at 50% behind the field.
- `FONT` — const
- `HELPER_SIDE` — const
- `HELPER_TOP` — `FormHelperText.js`, `size: 'small'`: `marginTop: 4`; contained: `marginLeft: 14`, `marginRight: 14`.
- `ICON` — const
- `ICON_GAP` — `InputAdornment.js`: `marginRight: 8`.

## Depends on

- [[Tokens Module]] — uses `easing`, `spec`, `states`
- [[Base Module]] — uses `new`
- [[Motion Module (mui)]] — uses `scaled`
- [[Curve Module]] — uses `EASE_OUT`
- [[Duration Module]] — uses `SHORTER`
- [[Glyph]] — uses `Glyph`
- [[Tone]] — uses `Tone`

## Used by

- [[Doc View Module]] — from `choices_row`, `draw`, `row`, `secret_control`
- [[codewiki-app]] — from `palette_view`, `toolbar`
- [[Select Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]

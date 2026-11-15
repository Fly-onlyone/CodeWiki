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
- `FONT` — const
- `ICON` — const
- `ICON_GAP` — `InputAdornment.js`: `marginRight: 8`.
- `OUTLINE_OPACITY` — `OutlinedInput.js`: the outline is the text color at 23%.
- `PADDING` — const

## Depends on

- [[Doc View Module]] — uses `id`
- [[Base Module]] — uses `new`
- [[Motion Module (mui)]] — uses `scaled`
- [[Tokens Module]] — uses `easing`
- [[Curve Module]] — uses `EASE_OUT`
- [[Duration Module]] — uses `SHORTER`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `palette_view`, `toolbar`

## See also

- [[_index]]
- [[codewiki-app]]

---
tags: [core, codewiki-app, module]
---

# Select Module

> `Select`, in MUI's outlined variant and small size.

## Source

- `crates/codewiki-app/src/mui/select.rs` — module `codewiki_app::mui::select`

## How it works

Main items:

- `Select` — A setting with a few named values, one of them in effect.
- `menu_width` — how wide the menu of a select is whose control is `control` wide.
- `menu` — the list of choices of a select, under the control at `anchor`: its rows, in one group or several, as the caller made them.
- `MENU_MIN_WIDTH` — the narrowest menu: a control cut short by a narrow window still opens a list its choices can be read in.
- `ARROW` — `SvgIcon.js`: an icon is 24 px.
- `ARROW_RIGHT` — `NativeSelectInput.js`, `NativeSelectIcon`, outlined: `right: 7`.
- `ARROW_ROOM` — `NativeSelectInput.js`, outlined: `paddingRight: 32`, the room the arrow takes.
- `FONT` — const

## Depends on

- [[Menu Module]] — uses `Menu`, `MenuSection`, `align`, `new`
- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Tokens Module]] — uses `easing`, `states`
- [[Motion Module (mui)]] — uses `scaled`
- [[Text Field Module]] — uses `HEIGHT`
- [[Curve Module]] — uses `EASE_OUT`
- [[Duration Module]] — uses `SHORTER`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `settings_menu_view`
- [[Doc View Module]] — from `select_control`

## See also

- [[_index]]
- [[codewiki-app]]

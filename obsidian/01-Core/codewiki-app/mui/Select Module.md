---
tags: [core, codewiki-app, module]
---

# Select Module

> `Select`, in MUI's outlined variant and small size.

## What it does

The closed control: the outline of a text field around the choice in effect, with an arrow at its end (`Select/SelectInput.js`, `NativeSelect/NativeSelectInput.js`, `OutlinedInput/OutlinedInput.js`). The outline's colors are those of `super::text_field`, which are TradingAgent's, so the two controls match side by side.

A click, Enter or Space asks for the list of choices, which is a `Menu` that hangs under the control, as wide as it; `menu` builds one. The owner keeps the menu's state, as for every menu.

MUI turns the arrow upside down while the list is open. The kit's icon cannot be turned, so the arrow stays as it is.

## Source

- `crates/codewiki-app/src/mui/select.rs` — module `codewiki_app::mui::select`

## How it works

1. `AppView::settings_menu_view` in [[codewiki-app]] calls `menu`. `menu` calls `Menu::new` in [[Menu Module]], then `menu_width`, then `px` in [[Zoom Module]].
2. `select_control` in [[Doc View Module]] calls `Select::new`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const PADDING: f32` — `OutlinedInput.js`, `size: 'small'`: `padding: '8.5px 14px'`.
- `const ARROW_ROOM: f32` — `NativeSelectInput.js`, outlined: `paddingRight: 32`, the room the arrow takes.
- `const ARROW_RIGHT: f32` — `NativeSelectInput.js`, `NativeSelectIcon`, outlined: `right: 7`.
- `const ARROW: f32` — `SvgIcon.js`: an icon is 24 px.
- `const ICON: f32` — the icon of the choice in effect: the size and the gap of a text field's start adornment (`text_field.rs`), so that the two controls match.
- `const OUTLINE_OPACITY: f32` — `OutlinedInput.js`: the outline is the text color at 23%.
- `const TEXT_DISABLED: (f32, f32)` — `styles/createPalette.js`, `text.disabled`, light and dark.
- `const MENU_MIN_WIDTH: f32` — the narrowest menu: a control cut short by a narrow window still opens a list its choices can be read in.
- `pub fn menu_width(control: f32) -> f32` — how wide the menu of a select is whose control is `control` wide. `SelectInput.js`: `minWidth` is the width of the control.
- `menu` — the list of choices of a select, under the control at `anchor`: its rows, in one group or several, as the caller made them. The row of the choice in effect is the one made `selected`.
- `pub struct Select` — A setting with a few named values, one of them in effect.

And 2 more private items with no comment.

## Depends on

- [[Menu Module]] — uses `Menu`, `MenuSection`, `align`, `new`
- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Tokens Module]] — uses `easing`, `states`
- [[Motion Module (mui)]] — uses `scaled`
- [[Text Field Module]] — uses `HEIGHT`
- [[Curve Module]] — uses `EASE_OUT`
- [[Duration Module]] — uses `SHORTER`
- [[Zoom Module]] — uses `px`

## Used by

- [[codewiki-app]] — from `settings_menu_view`
- [[Doc View Module]] — from `select_control`

## See also

- [[_index]]
- [[codewiki-app]]

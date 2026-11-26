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

There are two parts: the closed `Select` and the open list built by `menu`.

`Select::render` draws a text-field-shaped row: the optional icon, the label of the choice truncated, and a drop-down arrow 7 px from the end. The outline color eases over `duration::SHORTER` with the ease-out curve. It is primary when open or focused, primary at the hover opacity when hovered, the foreground at 23% at rest, and the disabled color when disabled. A second 1 px primary line, drawn as an inset shadow, makes the active border look thicker without moving the text. A focused but closed select also gets the focus ring.

A click calls `on_click`. The owner then opens the list.

`menu` builds a [[Menu Module]] aligned to the start of the anchor, with the sections the caller made. Its width is `menu_width` of the control's width, never under 160. The owner marks the row of the current choice as selected and keeps the open state, highlight and focus handle.

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
- [[Interaction Module]] — uses `current`
- [[Motion Module (mui)]] — uses `scaled`
- [[Text Field Module]] — uses `HEIGHT`
- [[Curve Module]] — uses `EASE_OUT`
- [[Duration Module]] — uses `SHORTER`

## Used by

- [[codewiki-app]] — from `settings_menu_view`
- [[Doc View Module]] — from `select_control`

## Gotchas

- The arrow does not turn upside down when the list opens, as MUI's does; the kit's icon cannot be rotated.
- A select does not ripple. The ink is fully transparent, since it is an input and not a button.
- The height is the text field's `HEIGHT`, so a select lines up with a text field in one row.
- The control has no state of its own. `open` only changes how it looks; the owner must set it while the menu is shown.
- A disabled select fades its icon and ignores a custom tint.
- The menu is at least 160 wide even if a narrow window squeezes the control.

## See also

- [[_index]]
- [[codewiki-app]]

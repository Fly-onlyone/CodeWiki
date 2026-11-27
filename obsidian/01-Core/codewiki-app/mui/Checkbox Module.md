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

`Checkbox` is a builder that you give an id, a `Check` state, an optional `edge_start` and an `on_click` handler. It does not hold the checked value; the owner decides what a click changes and passes the new state back on the next render.

`Look::of` picks the colors. The icon is the muted text color when unchecked, the primary color when checked or indeterminate, and the foreground at the disabled opacity when disabled. The hover tint is always the primary color at the app's own hover opacity.

On render, `Base::new` tracks hover and focus. The hover tint fades with a transition of `duration::SHORTEST` and the ease-in-out curve, scaled by the motion setting. The area is a round `div` of `SIZE` (38) with the icon (20) in the middle. A focused box gets the focus ring. The `Ink` ripple is centered and as large as the round area. `Check::icon` picks the blank, checked or indeterminate glyph.

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

- [[codewiki-app]] — from `review_queue`, `review_retire`
- [[Review Module]] — from `header`

## Gotchas

- All sizes go through the zoom module's `px`, not GPUI's, so the checkbox follows the app zoom.
- `Look::of` has a `disabled` flag, but `render` always passes false: the checkbox cannot be shown disabled today.
- The tint eases here, while MUI changes it at once. This is on purpose, to match the app's other controls.
- Use `edge_start` when the box, not its round area, must line up with the text above it. It pulls the control left by 3 pixels.
- A checkbox with no `on_click` still shows hover and focus, but does nothing.

## See also

- [[_index]]
- [[codewiki-app]]

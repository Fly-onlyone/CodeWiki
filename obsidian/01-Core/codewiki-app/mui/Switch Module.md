---
tags: [core, codewiki-app, module]
---

# Switch Module

> `Switch`.

## What it does

Sizes, opacities and timing are MUI's (`Switch/Switch.js` and `internal/SwitchBase.js`); TradingAgent does not restyle the switch. A track with a round thumb that slides along it; around the thumb a round area that tints under the pointer and carries the ripple.

In MUI an invisible input covers the whole switch, so a click anywhere on it toggles. Here the thumb's area is the control (hover, ripple, keyboard) and the track takes clicks of its own.

## Source

- `crates/codewiki-app/src/mui/switch.rs` — module `codewiki_app::mui::switch`

## How it works

1. `draw` in [[Doc View Module]] calls `Switch::new`.

## Items

- `const PADDING: f32` — `Switch.js`, `SwitchRoot`: `padding: 12` around the track.
- `pub const INSET: f32` — how far the track is from the switch's left edge. A row that lines the track up with what is above it pulls the switch left by this much, as MUI's `FormControlLabel` does (`marginLeft: -11`).
- `const TRACK_WIDTH: f32` — `Switch.js`, `SwitchRoot`: `width: 34 + 12 * 2`, `height: 14 + 12 * 2`.
- `pub const WIDTH: f32` — public constant; used by `Switch::render`.
- `pub const HEIGHT: f32` — public constant; used by `Switch::render`.
- `const THUMB: f32` — `Switch.js`, `SwitchThumb`: `width: 20`, `height: 20`.
- `const THUMB_PADDING: f32` — `SwitchBase.js`, `SwitchBaseRoot`: `padding: 9` around the thumb.
- `const HALO: f32` — the round area around the thumb.
- `const TRAVEL: f32` — `Switch.js`, `SwitchSwitchBase`, checked: `translateX(20px)`.
- `const TRACK_OFF: (f32, f32)` — `Switch.js`, `SwitchTrack`: `opacity` of an off track, light and dark.
- `const TRACK_ON: f32` — `Switch.js`, checked: the track at `opacity: 0.5`, in the primary color.
- `const TRACK_DISABLED: (f32, f32)` — `Switch.js`, disabled: the track's `opacity`, light and dark.
- `const DISABLED_LIGHTEN: f32` — `Switch.js`, disabled and checked: `lighten(main, 0.62)` on a light theme, `darken(main, 0.55)` on a dark one.
- `const THUMB_OFF: (f32, f32)` — lightness of the greys MUI gives the thumb (`colors/grey.js`): `common.white` off on a light theme and `grey[300]` on a dark one; disabled, `grey[100]` and `grey[600]`. They are greys in every theme: the thumb of an off switch carries no hue.
- `fn lighten(color: Hsla, amount: f32) -> Hsla` — MUI's `lighten` and `darken` (`@mui/system`, `colorManipulator.js`): each channel moves toward white, or toward black, by `amount`.
- `pub fn halo_left(progress: f32) -> f32` — how far the thumb's area is from the left of the switch, for a thumb `progress` of the way from off (0) to on (1).
- `pub struct Look` — the colors of a switch in one state.
- `type OnChange = Rc<dyn Fn(bool, &mut Window, &mut App)>` — what a change of the switch does; it is given the new value.
- `pub struct Switch` — A setting that is on or off, and takes effect when it is clicked.

And 6 more private items with no comment.

## Depends on

- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Tokens Module]] — uses `easing`, `elevation`
- [[Interaction Module]] — uses `current`
- [[Motion Module (mui)]] — uses `scaled`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `SHORTEST`
- [[Zoom Module]] — uses `px`

## Used by

- [[Doc View Module]] — from `draw`

## See also

- [[_index]]
- [[codewiki-app]]

---
tags: [core, codewiki-app, module]
---

# Switch Module

> `Switch`.

## Source

- `crates/codewiki-app/src/mui/switch.rs` — module `codewiki_app::mui::switch`

## How it works

Main items:

- `Switch` — A setting that is on or off, and takes effect when it is clicked.
- `Look` — the colors of a switch in one state.
- `halo_left` — how far the thumb's area is from the left of the switch, for a thumb `progress` of the way from off (0) to on (1).
- `HEIGHT` — const
- `WIDTH` — const
- `INSET` — how far the track is from the switch's left edge.
- `PADDING` — `Switch.js`, `SwitchRoot`: `padding: 12` around the track.
- `TRAVEL` — `Switch.js`, `SwitchSwitchBase`, checked: `translateX(20px)`.

## Depends on

- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Tokens Module]] — uses `easing`, `elevation`, `states`
- [[Motion Module (mui)]] — uses `scaled`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `SHORTEST`
- [[Entry]] — uses `size`

## Used by

- [[Doc View Module]] — from `draw`

## See also

- [[_index]]
- [[codewiki-app]]

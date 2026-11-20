---
tags: [core, codewiki-app, module]
---

# Backdrop Module

> The drifting backdrop: soft color under the glass.

## Source

- `crates/codewiki-app/src/backdrop.rs` — module `codewiki_app::backdrop`

## How it works

Main items:

- `Shell` — the window's outermost view: the drifting shapes, and `content` over them.
- `STRENGTH` — how strongly a shape colors the backdrop at its middle.
- `TitleBarFn` — draws the window's title bar.
- `RINGS` — discs in one shape.
- `Blob` — one shape: where it rests, how far it wanders, and how fast.
- `MICROSOFT_VENDOR` — how GPUI names an adapter from Microsoft's vendor ID, 0x1414.
- `advance` — `phase` moved on by `seconds`, wrapped into 0 to 1.
- `MAX_STEP` — the longest step one frame may take, so that a stalled frame does not show as a jump.

## Depends on

- [[Doc View Module]] — uses `DocPage`, `PageSlot`, `bounds`, `read_by`
- [[Glass Module]] — uses `debug_backdrop`
- [[Themes Module]] — uses `of`
- [[Entry]] — uses `size`

## Used by

- [[codewiki-app]] — from `main`
- [[Glass Module]] — from `matched_backdrop`

## See also

- [[_index]]
- [[codewiki-app]]

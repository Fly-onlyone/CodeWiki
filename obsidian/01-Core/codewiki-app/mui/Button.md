---
tags: [core, codewiki-app]
---

# Button

> A button with a label, and optionally an icon before it.

## Source

- `crates/codewiki-app/src/mui/button.rs` — definition of `Button`

## How it works

Declared as `pub struct Button`.

Public API:

- `pub fn icon(mut self, icon: Glyph) -> Self` — MUI's `startIcon`.
- `pub fn new(id: impl Into<ElementId>) -> Self`
- `pub fn text(mut self) -> Self`
- `pub fn contained(mut self) -> Self`
- `pub fn error(mut self) -> Self`
- `pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self`

## Depends on

- [[Button Module]] — uses `HOVER_TIME`, `ICON_GAP`, `Kind`, `LIFT`
- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Tokens Module]] — uses `spec`, `states`
- [[Doc View Module]] — uses `id`
- [[Feedback Module]] — uses `tooltip`
- [[Motion Module (mui)]] — uses `SNAPPY`
- [[Glyph]] — uses `Glyph`
- [[Spring]] — uses `policy`

## Used by

- [[codewiki-app]] — from `details`, `edit_view`, `proposal_actions`, `toolbar`
- [[Doc View Module]] — from `draw`

## See also

- [[_index]]
- [[Button Module]]

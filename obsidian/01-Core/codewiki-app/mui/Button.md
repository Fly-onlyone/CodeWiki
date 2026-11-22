---
tags: [core, codewiki-app]
---

# Button

> A button with a label, and optionally an icon before it.

## Source

- `crates/codewiki-app/src/mui/button.rs` — definition of `Button`

## How it works

1. `AppView::details` and `AppView::diagram_viewer_view` in [[codewiki-app]], and 7 more, call `new`.

## Items

Declared as `pub struct Button`.

Members:

- `pub fn new(id: impl Into<ElementId>) -> Self` — public method, returns `Self`; called by `AppView::details`, `AppView::diagram_viewer_view`, `AppView::edit_view`, `AppView::proposal_actions` and 5 more.
- `pub fn label(mut self, label: impl Into<SharedString>) -> Self` — public method, returns `Self`.
- `pub fn icon(mut self, icon: Glyph) -> Self` — MUI's `startIcon`.
- `pub fn contained(mut self) -> Self` — public method, returns `Self`; called by `draw`.
- `pub fn text(mut self) -> Self` — public method, returns `Self`; called by `Button::render`, `Button::tooltip`.
- `pub fn error(mut self) -> Self` — public method, returns `Self`.
- `pub fn small(mut self) -> Self` — public method, returns `Self`.
- `pub fn disabled(mut self, disabled: bool) -> Self` — public method, returns `Self`.
- `pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self` — public method, returns `Self`; called by `Button::render`.
- `on_click` — public method, returns `Self`.
- `fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement` — public method, returns `impl IntoElement`.

## Depends on

- [[Button Module]] — uses `HOVER_TIME`, `ICON_GAP`, `Kind`, `LIFT`
- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Tokens Module]] — uses `spec`, `states`
- [[Feedback Module]] — uses `tooltip`
- [[Motion Module (mui)]] — uses `SNAPPY`
- [[Zoom Module]] — uses `px`
- [[Glyph]] — uses `Glyph`
- [[Spring]] — uses `policy`

## Used by

- [[codewiki-app]] — from `details`, `diagram_viewer_view`, `edit_view`, `proposal_actions`
- [[Doc View Module]] — from `draw`, `draw_row`, `secret_control`

## See also

- [[_index]]
- [[Button Module]]

---
tags: [core, codewiki-app]
---

# Tone

> What kind of news something is.

## What it does

MUI's `severity`, and the `color` of a chip.

## Source

- `crates/codewiki-app/src/mui/feedback.rs` — definition of `Tone`

## Items

Declared as `pub enum Tone`.

Members:

- `pub fn color(self, cx: &App) -> Hsla` — the color that stands for this tone.
- `pub fn icon(self) -> Glyph` — the icon an alert of this tone starts with.

## Depends on

- [[Glyph]] — uses `Glyph`

## Used by

- [[Doc Module]] — from `Cell`, `Stat`, `chip`, `tone`
- [[Doc View Module]] — from `caption`, `row`, `setting_row`, `under`
- [[Feedback Module]] — from `alert`, `alert_titled`, `chip`
- [[Text Field Module]] — from `TextField`, `helper`
- [[Illustration Module]] — from `tone_of`
- [[Snackbar Module]] — from `Content`
- [[Settings Pages Module]] — from `job`
- [[Toasts Module]] — from `Notice`

## See also

- [[_index]]
- [[Feedback Module]]

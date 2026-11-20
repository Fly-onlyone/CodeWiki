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
- `fn icon(self) -> Glyph` — private method, returns `Glyph`.

## Depends on

- [[Glyph]] — uses `Glyph`

## Used by

- [[Doc Module]] — from `Cell`, `Stat`, `chip`, `tone`
- [[Doc View Module]] — from `caption`, `row`, `setting_row`, `under`
- [[Feedback Module]] — from `alert`, `chip`
- [[Text Field Module]] — from `TextField`, `helper`
- [[Illustration Module]] — from `tone_of`
- [[Settings Pages Module]] — from `job`
- [[Block]] — from `Block`

## See also

- [[_index]]
- [[Feedback Module]]

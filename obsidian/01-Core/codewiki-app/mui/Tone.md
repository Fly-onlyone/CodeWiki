---
tags: [core, codewiki-app]
---

# Tone

> What kind of news something is.

## Source

- `crates/codewiki-app/src/mui/feedback.rs` — definition of `Tone`

## How it works

Declared as `pub enum Tone`.

MUI's `severity`, and the `color` of a chip.

Public API:

- `pub fn color(self, cx: &App) -> Hsla` — the color that stands for this tone.

## Depends on

- [[Glyph]] — uses `Glyph`

## Used by

- [[Doc Module]] — from `Cell`, `Stat`, `chip`, `tone`
- [[Feedback Module]] — from `alert`, `chip`
- [[Illustration Module]] — from `tone_of`
- [[Block]] — from `Block`

## See also

- [[_index]]
- [[Feedback Module]]

---
tags: [core, codewiki-app]
---

# Tone

> What kind of news something is.

## What it does

`Tone` is the one word the app uses for what kind of news a piece of UI carries: neutral, tertiary, success, info, warning or error. It is MUI's `severity` for an alert and the `color` of a chip. Alerts, chips, toasts, text field helper lines, document cells and illustrations all take a `Tone` and so share the same colors.

## Source

- `crates/codewiki-app/src/mui/feedback.rs` — definition of `Tone`

## How it works

`Tone::color` looks up the theme at call time: neutral is the muted text color, tertiary is the sidebar primary color, and success, info, warning and error are the theme's own. A theme change therefore changes every tone with no extra work.

`Tone::icon` picks the alert icon: a success, warning or error icon, and the info icon for the other three.

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

## Gotchas

- `Neutral` is the default. A piece that is given no tone looks muted.
- `Tertiary` is for labels that are no news, such as the kind of a thing. Do not use it for a status.
- `Neutral`, `Tertiary` and `Info` share one icon, so the icon alone cannot tell them apart; the color does.
- The color needs the app context, so it is read at draw time and must not be cached across a theme change.

## See also

- [[_index]]
- [[Feedback Module]]

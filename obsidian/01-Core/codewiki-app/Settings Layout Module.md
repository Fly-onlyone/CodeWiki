---
tags: [core, codewiki-app, module]
---

# Settings Layout Module

> The measures of the Settings page that are a matter of taste.

## What it does

This file holds the sizes of the Settings page that are chosen by eye. Sizes copied from Material UI or TradingAgent live in the `mui` folder beside their component, with their source noted. The numbers here have no source: the width of the category list, how a setting's row is split, the room around the entry button. Keeping them in one place means they can be tuned together.

## Source

- `crates/codewiki-app/src/settings_layout.rs` — module `codewiki_app::settings_layout`

## How it works

The file is only constants, all in logical pixels. The page frame (`AppView::settings_frame`) uses the list width, header height and pane padding, and caps the page at the pane measure. Each setting row (`row` in [[Doc View Module]]) is a label column, a gap and a control column, from the label width, row gap and control width. The row spacing separates rows inside one card. The entry padding spaces the Settings button at the foot of the sidebar and of Home.

## Items

- `pub const CATEGORY_LIST_WIDTH: f32` — the list of categories at the left of the page.
- `pub const CATEGORY_LIST_PADDING: f32` — room around the list's rows, and above its first one.
- `pub const HEADER_HEIGHT: f32` — the line with the Back button and the word "Settings", above the list.
- `pub const PANE_PADDING: f32` — room around a category's page, and the widest the page gets.
- `pub const PANE_MEASURE: f32` — public constant; used by `AppView::settings_frame`.
- `pub const LABEL_WIDTH: f32` — A row of one setting: its name and help at the left, its control at the right, and the room between the two.
- `pub const CONTROL_WIDTH: f32` — public constant; used by `row`.
- `pub const ROW_GAP: f32` — public constant; used by `row`.
- `pub const ROW_SPACING: f32` — between the rows of one card.
- `pub const ENTRY_PADDING: f32` — the "Settings" button at the foot of the sidebar and of Home: the room around it.

## Used by

- [[codewiki-app]] — from `settings_entry`, `settings_frame`
- [[Doc View Module]] — from `row`

## Gotchas

- Label width, row gap and control width add up to 624, which fits in the 760 measure with room for the page padding. Change one and check the others.
- A value that has a source in MUI belongs in `mui`, not here.

## See also

- [[_index]]
- [[codewiki-app]]

---
tags: [core, codewiki-app, module]
---

# Settings Layout Module

> The measures of the Settings page that are a matter of taste.

## What it does

Numbers copied from Material UI or TradingAgent stay in `mui/`, beside the component they belong to, with their source. What is here has no source to copy: how wide the category list is, how a row of a setting is divided, how much room the entry button takes. They are in one place so that they can be tuned by eye and changed together.

## Source

- `crates/codewiki-app/src/settings_layout.rs` — module `codewiki_app::settings_layout`

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

## See also

- [[_index]]
- [[codewiki-app]]

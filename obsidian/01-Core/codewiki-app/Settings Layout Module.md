---
tags: [core, codewiki-app, module]
---

# Settings Layout Module

> The measures of the Settings page that are a matter of taste.

## Source

- `crates/codewiki-app/src/settings_layout.rs` — module `codewiki_app::settings_layout`

## How it works

Main items:

- `CONTROL_WIDTH` — const
- `LABEL_WIDTH` — A row of one setting: its name and help at the left, its control at the right, and the room between the two.
- `ROW_GAP` — const
- `ROW_SPACING` — between the rows of one card.
- `ENTRY_PADDING` — the "Settings" button at the foot of the sidebar and of Home: the room around it.
- `CATEGORY_LIST_PADDING` — room around the list's rows, and above its first one.
- `CATEGORY_LIST_WIDTH` — the list of categories at the left of the page.
- `HEADER_HEIGHT` — the line with the Back button and the word "Settings", above the list.

## Used by

- [[codewiki-app]] — from `settings_entry`, `settings_frame`
- [[Doc View Module]] — from `row`

## See also

- [[_index]]
- [[codewiki-app]]

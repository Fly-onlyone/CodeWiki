---
tags: [core, codewiki-app, module]
---

# Tabs Module

> `Tabs` and `Tab`.

## Source

- `crates/codewiki-app/src/mui/tabs.rs` — module `codewiki_app::mui::tabs`

## How it works

Main items:

- `Tabs` — A row of tabs with a bar under the selected one.
- `Tab` — one tab: its label and, optionally, something after it, such as a close button.
- `bar` — where the bar sits under a tab that starts at `left` and is `width` wide: its own left edge and width.
- `HEIGHT` — `MuiTab`: `minHeight: 44`.
- `BAR_MAX` — const
- `BAR_SHARE` — const
- `OnSelect` — what choosing the tab at an index does.
- `BAR` — `MuiTab`, `::after`: `height: 3`, `width: 60%`, `maxWidth: 88`.

## Depends on

- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Doc View Module]] — uses `id`
- [[Motion Module (mui)]] — uses `SNAPPY`
- [[Spring]] — uses `policy`

## Used by

- [[codewiki-app]] — from `tab_bar`
- [[Illustration Module]] — from `tabs`

## See also

- [[_index]]
- [[codewiki-app]]

---
tags: [core, codewiki-app, module]
---

# Illustration Module

> Native renderer for illustrations.

## Source

- `crates/codewiki-app/src/illustration.rs` — module `codewiki_app::illustration`

## How it works

Main items:

- `render` — draw one node of an illustration.
- `Props` — type alias
- `list_of` — function
- `text_of` — function
- `label` — function
- `numbers` — function
- `series_color` — series colors, cycled.
- `bar_chart` — horizontal bars: one row per label, one bar per series.

## Depends on

- [[Node]] — uses `Node`, `as_list`

## Used by

- [[codewiki-app]] — from `viewer`

## See also

- [[_index]]
- [[codewiki-app]]

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
- `children` — `path` names each node by its place in the tree, so that a component that remembers something, such as the chosen tab, keeps it between frames.

## Depends on

- [[Surfaces Module]] — uses `card`, `card_content`, `divider`, `text`
- [[Doc Module]] — uses `hint`, `new`, `text`
- [[Doc View Module]] — uses `plain_table`, `stat_tile`
- [[Motion Module (mui)]] — uses `TAB`, `enter`
- [[Tabs Module]] — uses `new`, `tabs`
- [[Node]] — uses `Node`, `as_list`
- [[Feedback Module]] — uses `chip`
- [[Tone]] — uses `Tone`

## Used by

- [[codewiki-app]] — from `viewer`

## See also

- [[_index]]
- [[codewiki-app]]

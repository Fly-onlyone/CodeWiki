---
tags: [core, codewiki-app, module]
---

# Doc View Module

> Draws a `Doc` with the `mui` components.

## Source

- `crates/codewiki-app/src/doc_view.rs` — module `codewiki_app::doc_view`

## How it works

Main items:

- `PageSlot` — where in the window the page goes.
- `DocPage` — A generated page as a view of its own, so that it can be drawn cached.
- `OnAction` — what a link, row or button of a page does when used.
- `notify_after_draw` — ask for `view` to be drawn again, from a place that runs while a frame is being drawn.
- `stat_tile` — TradingAgent's metric tile: a label, a large figure that counts up when the tile appears, and a note under it.
- `plain_table` — A table outside a page, such as one in an illustration: no links, and named by `key` among the other things drawn that frame.
- `Page` — what every block needs to draw itself.
- `Strip` — one strip of a page, as the list scrolls it: the title, a whole block, or a piece of a long table.

## Depends on

- [[Motion Module (mui)]] — uses `EnterExt`, `GENTLE`, `STAGGER_CAP`, `count_up`
- [[Doc Module]] — uses `Cell`, `Choice`, `Row`, `Stat`
- [[Surfaces Module]] — uses `CARD_PADDING`, `card`, `card_content`, `card_piece`
- [[Feedback Module]] — uses `alert`, `alert_action`, `chip`, `progress`
- [[ListItemButton]] — uses `ListItemButton`, `new`, `secondary`
- [[Enter]] — uses `PAGE`, `card`, `row`
- [[Tokens Module]] — uses `easing`, `spec`
- [[Reading Module]] — uses `WIDE_MEASURE`, `gutter`

## Used by

- [[codewiki-app]] — from `AppView`, `doc_element`, `edit_view`, `graph_view`
- [[Backdrop Module]] — from `Shell`, `new`, `render`
- [[Illustration Module]] — from `render`, `table`
- [[Button Module]] — from `render`
- [[Menu Module]] — from `track`
- [[Tabs Module]] — from `render`
- [[Text Field Module]] — from `render`
- [[Button]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]

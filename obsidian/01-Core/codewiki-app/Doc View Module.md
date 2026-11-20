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
- `SelectRequest` — A select that was clicked: where it is, and what it offers.
- `notify_after_draw` — ask for `view` to be drawn again, from a place that runs while a frame is being drawn.
- `OnAction` — what a link, row or button of a page does when used.
- `Controls` — what the blocks that hold live controls need from the view that owns them: the text inputs, and the list of a select.
- `OnSelect` — what a click on a select does: its owner opens the list.
- `column` — A short page drawn whole, with live controls: a category of the settings.

## Depends on

- [[Doc Module]] — uses `Cell`, `Choice`, `Row`, `SecretState`
- [[Motion Module (mui)]] — uses `EnterExt`, `GENTLE`, `STAGGER_CAP`, `count_up`
- [[Feedback Module]] — uses `alert`, `alert_action`, `chip`, `linear_progress`
- [[Surfaces Module]] — uses `CARD_PADDING`, `card`, `card_content`, `card_piece`
- [[Switch Module]] — uses `INSET`, `new`, `on`, `on_change`
- [[Settings Layout Module]] — uses `CONTROL_WIDTH`, `LABEL_WIDTH`, `ROW_GAP`, `ROW_SPACING`
- [[Button Module]] — uses `contained`, `error`, `new`
- [[Text Field Module]] — uses `HEIGHT`, `helper`, `new`

## Used by

- [[codewiki-app]] — from `AppView`, `doc_element`, `graph_view`, `new`
- [[Backdrop Module]] — from `Shell`, `new`, `render`
- [[Illustration Module]] — from `render`, `table`
- [[Menu Module]] — from `track`
- [[Win32 Module]] — from `on_console_stop`

## See also

- [[_index]]
- [[codewiki-app]]

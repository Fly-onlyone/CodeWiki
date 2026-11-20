---
tags: [core, codewiki-app, module]
---

# Tabs Module

> `Tabs` and `Tab`.

## What it does

Sizes are TradingAgent's (`components.ts`, `MuiTab`): 44 px high, 8 px of padding each side, 4 px between tabs, and under the selected tab a bar three pixels thick and 60% of the tab wide. TradingAgent scales that bar in from its middle. GPUI cannot scale an element, so the bar slides from the old tab to the new one on a spring instead, as the indicator of MUI itself slides (`Tabs/Tabs.js`).

## Source

- `crates/codewiki-app/src/mui/tabs.rs` — module `codewiki_app::mui::tabs`

## How it works

1. `AppView::tab_bar` in [[codewiki-app]] and `tabs` in [[Illustration Module]] call `Tab::new`.
2. `AppView::tab_bar` in [[codewiki-app]] and `tabs` in [[Illustration Module]] call `Tabs::new`.

## Items

- `pub const HEIGHT: f32` — `MuiTab`: `minHeight: 44`.
- `const PADDING: f32` — `MuiTab`: `paddingLeft: 8`, `paddingRight: 8`.
- `const GAP: f32` — `MuiTabs`, `flexContainer`: `gap: 4`.
- `const FONT: f32` — `Tab/Tab.js`: the button text size at a line height of 1.25.
- `const BAR: f32` — `MuiTab`, `::after`: `height: 3`, `width: 60%`, `maxWidth: 88`.
- `const LABEL_MAX: f32` — the widest a label gets before it is cut short.
- `pub fn bar(left: f32, width: f32) -> (f32, f32)` — where the bar sits under a tab that starts at `left` and is `width` wide: its own left edge and width.
- `type OnSelect = Rc<dyn Fn(usize, &mut Window, &mut App)>` — what choosing the tab at an index does.
- `pub struct Tab` — one tab: its label and, optionally, something after it, such as a close button.
- `pub struct Tabs` — A row of tabs with a bar under the selected one.

And 3 more private items with no comment.

## Depends on

- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Motion Module (mui)]] — uses `SNAPPY`
- [[Zoom Module]] — uses `px`
- [[Spring]] — uses `policy`

## Used by

- [[codewiki-app]] — from `tab_bar`
- [[Illustration Module]] — from `tabs`

## See also

- [[_index]]
- [[codewiki-app]]

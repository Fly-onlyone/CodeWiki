---
tags: [core, codewiki-app, module]
---

# Tabs Module

> `Tabs` and `Tab`.

## What it does

A row of tabs for the app's tab bar. Each `Tab` has a label and an optional suffix, such as a close button. `Tabs` draws them with a bar under the selected one and calls back with the index when one is chosen.

Sizes follow TradingAgent: 44 px high, 8 px padding, 4 px gap. The selected tab is tinted and semibold. A tab under the pointer is tinted too, and its muted label brightens toward the text color. The strengths come from [[Interaction Module]]. The bar slides from the old tab to the new one on a spring, because GPUI cannot scale an element in from its middle.

## Source

- `crates/codewiki-app/src/mui/tabs.rs` — module `codewiki_app::mui::tabs`

## How it works

The row cannot know where its tabs are before they are laid out. `Tabs::render` keeps a keyed state of each tab's left edge and width. `on_children_prepainted` measures the tabs after each paint and stores the result, and notifies only when it changed.

On the next frame, the selected tab's measure goes through `bar`, which makes a bar 60% of the tab wide, at most 88 px, centred. Two springs (`bar-left`, `bar-width`) move the bar there. Until the first measure exists, no bar is drawn.

Each tab gets its own `Base` (see [[Base Module]]). Its fill is primary at the selected strength plus the hover strength, eased by a transition, then raised by `pressed` while pressed. Its label color is primary when selected, else the muted color blended toward the foreground by hover progress.

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
- [[Interaction Module]] — uses `current`, `pressed`
- [[Motion Module (mui)]] — uses `SNAPPY`, `scaled`
- [[Tokens Module]] — uses `easing`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `SHORTEST`
- [[Zoom Module]] — uses `px`
- [[Spring]] — uses `policy`

## Used by

- [[codewiki-app]] — from `tab_bar`

## Gotchas

- The bar appears one frame late, after the first measure. A selected index with no measured tab draws no bar.
- The bar is the last child of the row. The measuring code takes only the first `count` children so the bar is not counted as a tab.
- Labels are cut at 180 px.
- A tab does not size itself to the bar: a very narrow tab still gets a bar inside its own bounds, never outside.

## See also

- [[_index]]
- [[codewiki-app]]

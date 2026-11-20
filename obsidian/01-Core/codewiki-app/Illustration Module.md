---
tags: [core, codewiki-app, module]
---

# Illustration Module

> Native renderer for illustrations.

## What it does

Turns an evaluated OpenUI Lang tree into GPUI elements, drawn with the `mui` components. Every component of the CodeWiki library (`cw_illustrate::library`) has a branch here; a component this renderer does not know is drawn as its name, never dropped silently.

## Source

- `crates/codewiki-app/src/illustration.rs` — module `codewiki_app::illustration`

## How it works

1. `AppView::page_content` in [[codewiki-app]] calls `render`. `render` calls `text` in [[Surfaces Module]], then `label`, then `card_content` in [[Surfaces Module]], then `text_of`, then `children`, then `list_of`, and 13 more. `children` calls `render`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const CELL_GAP: f32` — space around each cell of a grid: Tailwind's `p-1`, as before.
- `const STEP_ICON: f32` — MUI's `StepIcon`: a 24 px disc with the step's number in 12 px text.
- `fn text_of(props: &Props, key: &str) -> Option<String>` — private function, returns `Option<String>`; called by `bar_chart`, `pie_chart`, `render`, `steps` and 2 more.
- `fn list_of<'a>(props: &'a Props, key: &str) -> &'a [Node]` — private function, returns `&'a [Node]`; called by `bar_chart`, `pie_chart`, `render`, `steps` and 2 more.
- `fn label(node: &Node) -> String` — private function, returns `String`; called by `bar_chart`, `pie_chart`, `render`, `table`.
- `fn series_color(index: usize, cx: &App) -> Hsla` — series colors, cycled.
- `fn tone_of(tone: Option<&str>) -> Tone` — the tone a component names, as the `mui` components know it.
- `fn children(nodes: &[Node], path: &str, window: &mut Window, cx: &mut App) -> Vec<AnyElement>` — `path` names each node by its place in the tree, so that a component that remembers something, such as the chosen tab, keeps it between frames.
- `fn bar_chart(props: &Props, cx: &App) -> AnyElement` — horizontal bars: one row per label, one bar per series.
- `fn pie_chart(props: &Props, cx: &App) -> AnyElement` — A pie is drawn as shares: label, proportional bar, percentage.
- `fn tabs(props: &Props, path: &str, window: &mut Window, cx: &mut App) -> AnyElement` — tabs: one panel shown at a time, the one whose tab was chosen.
- `fn steps(props: &Props, cx: &App) -> AnyElement` — MUI's vertical `Stepper`: numbered discs joined by a line.
- `pub fn render(node: &Node, path: &str, window: &mut Window, cx: &mut App) -> AnyElement` — draw one node of an illustration. `path` names its place in the tree.

And 4 more private items with no comment.

## Depends on

- [[Surfaces Module]] — uses `card`, `card_content`, `divider`, `text`
- [[Doc Module]] — uses `hint`, `new`, `text`
- [[Doc View Module]] — uses `TILE_MIN`, `plain_table`, `stat_tile`
- [[Tabs Module]] — uses `new`, `tabs`
- [[Node]] — uses `Node`, `as_list`
- [[Feedback Module]] — uses `chip`
- [[Motion Module (mui)]] — uses `enter`
- [[Zoom Module]] — uses `px`

## Used by

- [[codewiki-app]] — from `page_content`

## See also

- [[_index]]
- [[codewiki-app]]

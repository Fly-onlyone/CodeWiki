---
tags: [core, codewiki-app, module]
---

# Illustration Module

> Native renderer for the pictures of a note.

## What it does

Turns an evaluated OpenUI Lang tree into GPUI elements, drawn with the `mui` components. Every component of the CodeWiki library (`cw_illustrate::library`) has a branch here; a component this renderer does not know is drawn as its name, never dropped silently.

A picture explains what the text beside it says: a `Flow` is a row of numbered boxes joined by arrows, read from a cause to its result, and a `Callout` is a marked key point.

## Source

- `crates/codewiki-app/src/illustration.rs` — module `codewiki_app::illustration`

## How it works

1. `AppView::page_content` in [[codewiki-app]] calls `render`. `render` calls `list_of`, then `flow`, then `step`, then `alert_titled` in [[Feedback Module]], then `tone_of`, then `text_of`, and 1 more. `flow` calls `list_of`, then `px` in [[Zoom Module]], then `step`, then `text_of`, then `text` in [[Surfaces Module]], then `rows`. `step` calls `card` in [[Surfaces Module]], then `px` in [[Zoom Module]], then `text` in [[Surfaces Module]], then `text_of`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const STEP_ICON: f32` — MUI's `StepIcon`: a 24 px disc with the step's number in 12 px text.
- `const STEP_MIN: f32` — the narrowest a step's box gets before its row wraps. Three boxes of this width and their arrows fit the note's column.
- `const ROW_STEPS: usize` — boxes in one row at most. A longer flow takes several rows.
- `const ARROW: f32` — the arrow that leads to a step, and the room it stands in.
- `const ROW_GAP: f32` — space between two rows of a flow that wrapped.
- `const STEP_PADDING: f32` — inside a step's box.
- `fn text_of(props: &Props, key: &str) -> Option<String>` — private function, returns `Option<String>`; called by `flow`, `render`, `step`.
- `fn list_of<'a>(props: &'a Props, key: &str) -> &'a [Node]` — private function, returns `&'a [Node]`; called by `flow`, `render`.
- `fn tone_of(tone: Option<&str>) -> Tone` — the tone a callout names, as the `mui` components know it. One without a tone is a plain piece of information.
- `fn step(number: usize, props: &Props, cx: &App) -> Div` — one box of a flow: its number, its title, and a sentence under them.
- `fn rows(steps: usize) -> Vec<usize>` — how many boxes stand in each row of a flow of `steps` boxes: as few rows as `ROW_STEPS` allows, and the boxes spread evenly over them, so that no row ends with one box alone. Four steps are two rows of two.
- `fn flow(props: &Props, cx: &App) -> AnyElement` — A flow: boxes in rows, each box but the last followed by the arrow that leads on from it. The arrow after a row's last box says that the flow goes on in the next row; the numbers carry the order. Every box has the room of an arrow after it, so the boxes of all rows stand in line. In a pane narrower than the note's column a row wraps by itself.
- `pub fn render(node: &Node, cx: &App) -> AnyElement` — draw one node of a picture.

And 2 more private items with no comment.

## Depends on

- [[Surfaces Module]] — uses `card`, `text`
- [[Node]] — uses `Node`, `as_list`
- [[Feedback Module]] — uses `alert_titled`
- [[Zoom Module]] — uses `px`
- [[Tone]] — uses `Tone`

## Used by

- [[codewiki-app]] — from `page_content`

## See also

- [[_index]]
- [[codewiki-app]]

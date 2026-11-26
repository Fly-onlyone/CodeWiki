---
tags: [core, codewiki-app, module]
---

# Illustration Module

> Native renderer for the pictures of a note.

## What it does

A note can carry pictures: a flow of boxes and arrows, or a marked callout. A picture is stored as a small program in OpenUI Lang (see [[openui-lang]]). This module draws the evaluated tree of such a program as native GPUI elements, using the app's `mui` components. It is how a picture reaches the screen in the note view.

## Source

- `crates/codewiki-app/src/illustration.rs` — module `codewiki_app::illustration`

## How it works

`AppView::page_content` calls `render` with the root node of a picture. `render` matches the component name. A `Picture` becomes a column of its children, drawn by calling `render` again. A `Flow` goes to `flow`. A `Step` outside a flow draws its box alone. A `Callout` becomes an alert with a tone and an optional title, through `alert_titled`. Any other name is drawn as `[Name]` in muted text, so an unknown component is visible, not dropped.

`flow` collects its steps and lets `rows` split them into rows of at most three, spread evenly, so four steps are two rows of two. Each box is made by `step`: a card with a numbered disc, the title, and a muted sentence. Every box has room for an arrow after it, and all but the last show one, so boxes of all rows line up. The arrow after the last box of a row says the flow goes on in the next row. A row also wraps by itself in a narrow pane.

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

## Gotchas

- `render` matches on component names. A component added to `cw_illustrate::library` needs a branch here, and a test fails until it has one.
- The tone `danger` maps to the app's `Error` tone. A missing or unknown tone is information.
- A row of three boxes, with their arrow rooms, must fit the note's reading column. A test checks this against the column width. Raise `STEP_MIN` or `ROW_STEPS` and it fails.
- Empty or whitespace-only text is treated as absent, so a blank step text draws no line.
- Sizes go through the zoom `px`, so pictures scale with the app's zoom.

## See also

- [[_index]]
- [[codewiki-app]]

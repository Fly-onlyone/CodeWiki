---
tags: [core, codewiki-app, module]
---

# Outline Module

> The outline in the details pane: the `##` headings of the note as its body draws them, which of their sections is being read, and the glide of the body to a heading the user clicks.

## What it does

Places are in design units, measured down from the top of the note: what the body's text view reports, divided by one zoomed pixel.

## Source

- `crates/codewiki-app/src/outline.rs` — module `codewiki_app::outline`

## How it works

1. `AppView::outline_entries` in [[codewiki-app]] calls `entries`.
2. `AppView::outline_entries` in [[codewiki-app]] calls `spy`.
3. `AppView::jump_to_heading` and `AppView::step_outline` in [[codewiki-app]] call `target`.
4. `AppView::step_outline` in [[codewiki-app]] calls `glide_to`.
5. `AppView::step_outline` in [[codewiki-app]] calls `user_moved`.

## Items

- `pub const SPY_LINE: f32` — how far below the top of the body the line lies that decides which section is being read: a heading at or above it has begun its section.
- `pub type Place = (usize, f32)` — A place in the body as its list keeps it: a block and how far into it the view starts. The glide remembers the one it wrote, to tell it from a scroll of the user's.
- `pub fn entries(headings: Vec<DocumentHeading>) -> Vec<DocumentHeading>` — the entries of the outline: the `##` headings of the body.
- `pub fn spy(tops: &[f32], scroll: f32, max: f32, line: f32) -> Option<usize>` — the entry whose section is being read, from where each entry's heading starts (`tops`, in order), where the body is scrolled to and how far it can scroll (`max`). The last heading at or above `line` below the top lights its entry; none has before the first heading. At the very bottom the last entry lights, since a short last section never reaches the line.
- `pub fn target(top: f32, max: f32) -> f32` — where the body scrolls to so that a heading starting at `top` is at its top, or as far as the body goes when the note ends before that.
- `pub fn glide_to(from: f32, to: f32, progress: f32) -> f32` — where the body is `progress` of the way from `from` to `to`.
- `pub fn user_moved(written: Option<Place>, now: Place) -> bool` — whether the body is somewhere the glide did not put it: the user scrolled it, with the wheel, the scrollbar or by selecting text.
- `pub struct Jump` — A glide of the body to a heading, which keeps the heading's entry lit after it lands, until the user scrolls.

## Used by

- [[codewiki-app]] — from `AppView`, `jump_to_heading`, `outline_entries`, `step_outline`

## See also

- [[_index]]
- [[codewiki-app]]

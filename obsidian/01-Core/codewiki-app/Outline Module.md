---
tags: [core, codewiki-app, module]
---

# Outline Module

> The outline in the details pane: the `##` headings of the note as its body draws them, which of their sections is being read, and the glide of the body to a heading the user clicks.

## What it does

The details pane shows an outline of the note: a list of its `##` headings. This module holds the pure rules behind it, free of any UI. It picks the entries, decides which section the reader is in, works out where a click should scroll to, and tells a glide's scroll from the user's own. `AppView` in [[codewiki-app]] does the drawing and owns the state; this module does the arithmetic, so tests can cover it.

All places are in design units, measured down from the top of the note: the body's text view reports pixels, and the caller divides by one zoomed pixel.

## Source

- `crates/codewiki-app/src/outline.rs` — module `codewiki_app::outline`

## How it works

The body is a text view (`TextViewState`). The app owns it: the note's view lives in `Reader`, and while a note is edited the editor's preview is the body instead. `AppView::outline_entries` asks the view for its headings and `entries` keeps level 2 only. Headings inside code blocks or quotes are not counted, because the view already knows which blocks it drew.

### Which entry is lit

`outline_entries` lights the entry a glide is going to. With no glide, it reads each heading's top and the scroll position, and calls `spy`. `spy` returns the last heading at or above `SPY_LINE` below the top of the view. At the very bottom it returns the last entry, since a short last section never reaches the line. If the blocks are not laid out yet, the previous frame's answer is reused.

### Click and glide

`jump_to_heading` stores a `Jump`: where the body is now, where it should end (`target`, clamped to the scroll range), and the start time. Each frame `step_outline` computes progress with `mui::motion::glide`, maps it with `glide_to`, writes the offset, and asks for another frame until progress reaches 1. With reduced motion the body jumps at once. The heading's top is read again each frame, because a picture above it may still load and move it.

### Telling the user from the glide

The glide stores the place it wrote (`written`, a block and an offset into it). Next frame, `user_moved` compares that with where the body is. A difference over half a pixel, or another block, means the user scrolled, with the wheel, the scrollbar or a text selection. The `Jump` is dropped and `spy` takes over again. After landing, the `Jump` stays, so the clicked entry stays lit until the user moves.

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

## Gotchas

- A `Jump` is tied to one text view by `body`. If the page changes or editing starts, the ids differ and the jump is dropped.
- `spy` with `max` at zero (a note that fits its view) uses the line alone; the bottom rule applies only when the body can scroll.
- The clicked entry does not need to reach the line. A heading near the end of the note cannot scroll to the top, so `target` stops at the end and the entry stays lit through the jump.
- A jump is not put in the history: Back leaves the note, not the place in it.
- The 0.5 pixel tolerance in `user_moved` hides rounding in the list; a smaller value would end glides by mistake.

## See also

- [[_index]]
- [[codewiki-app]]

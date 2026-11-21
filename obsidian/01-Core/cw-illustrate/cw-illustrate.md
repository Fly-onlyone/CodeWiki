---
tags: [core, cw-illustrate]
---

# cw-illustrate

> The pictures of a CodeWiki note: their components, their limits and the file that keeps them.

## What it does

The pictures of a note: which components exist, what a picture may hold, and the file that keeps a note's pictures.

A picture explains what a note says: what happens, from a cause to its result, in plain words. It is written by a coding agent, as an OpenUI Lang program, and the app draws it natively; nothing is fetched or executed. Rules draw none: they know counts and calls, not what a thing is for.

This crate is the single definition of the components, so the check a submission must pass, the file the backend keeps and the app's renderer cannot drift apart.

## Source

- `crates/cw-illustrate/Cargo.toml` — package manifest
- `crates/cw-illustrate/src/lib.rs` — crate root

## How it works

Builds a library.

1. `get_proposal` in [[Api Module]] and `pictures_of` in [[Explanations Module]], and 2 more, call `parse_pictures`.
2. `AppView::open_note_as` in [[codewiki-app]] and `submit` in [[Explanations Module]] call `review`. `review` calls `check`, then `element`, then `sized`, then `text`.
3. `submit` in [[Explanations Module]] and `keep_placed_pictures` in [[Generate Module]] call `serialize_pictures`.
4. `brief` in [[Explanations Module]] calls `library`. `library` calls `Library::new` in [[Library]].
5. `AppView::open_note_as` in [[codewiki-app]] calls `check`. `check` calls `evaluate` in [[Eval Module]], then `parse` in [[Parser Module]], then `library`.

1 more entry point: `limits`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Defined at the crate root:

- `pub const EXTENSION: &str` — file extension of the file that keeps a note's pictures.
- `pub const MIN_STEPS: usize` — steps a flow has at least: one box explains nothing.
- `pub const MAX_STEPS: usize` — steps a flow has at most. More do not fit a reader's eye, nor the note's column.
- `pub const MAX_STEP_TITLE: usize` — characters of a step's title.
- `pub const MAX_STEP_TEXT: usize` — characters of a step's text.
- `pub const MAX_CALLOUT_TEXT: usize` — characters of a callout's text.
- `pub const MAX_CALLOUTS: usize` — callouts in one picture.
- `pub const MAX_SOURCE: usize` — bytes of one picture's source.
- `pub const MAX_PICTURES: usize` — pictures in one note.
- `pub const TONES: [&str; 4]` — what a callout can be marked as.
- `pub fn library() -> Library` — the components a picture may use.
  - `Picture(children)`: the picture. Its children are at most one `Flow` and at most `MAX_CALLOUTS` `Callout`s.
  - `Flow(steps, title?)`: boxes joined by arrows, read in order.
  - `Step(title, text?)`: one box of a flow.
  - `Callout(text, tone?, title?)`: a marked key point or warning; `tone` is one of `TONES`, `info` when left out.
- `pub fn check(source: &str) -> Document` — parse a picture and check it against the library.
- `pub fn quote(text: &str) -> String` — quote text as an OpenUI Lang string literal.
- `pub fn limits() -> String` — the limits of a picture, in words: for whoever writes one.
- `fn element(node: &Node) -> Option<(&str, &Props)>` — private function, returns `Option<(&str, &Props)>`; called by `review`.
- `fn text<'a>(props: &'a Props, key: &str) -> &'a str` — the text of the property `key`, and nothing for one left out.
- `pub fn review(source: &str) -> Vec<String>` — what is wrong with a picture, one line for each fault; empty when the picture may be kept. It must parse, use only the library's components, and stay within the limits: a picture is a few boxes of plain words, and one that is larger explains less.
- [[Picture]] — one picture of a note: the `## ` section it explains, and its source.
- `pub fn parse_pictures(file: &str) -> Vec<Picture>` — the pictures a pictures file holds, in its order. Text before the first mark belongs to no section and is left out.
- `pub fn serialize_pictures(pictures: &[Picture]) -> String` — the pictures file of `pictures`: each under a line that names its section. Empty for no pictures, in which case no file is kept.

And 3 more items.

## Depends on

- [[Node]] — uses `Node`, `as_list`, `as_text`
- [[Eval Module]] — uses `Document`, `evaluate`
- [[Library]] — uses `Library`, `new`
- [[Parser Module]] — uses `parse`
- [[Picture]] — uses `Picture`

## Used by

- [[Explanations Module]] — from `brief`, `pictures_of`, `submit`
- [[Generate Module]] — from `illustration_path`, `keep_placed_pictures`
- [[codewiki]] — from `picture_reference_follows_the_library`
- [[codewiki-app]] — from `open_note_as`
- [[Api Module]] — from `get_proposal`
- [[Query Module]] — from `read_note_as`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

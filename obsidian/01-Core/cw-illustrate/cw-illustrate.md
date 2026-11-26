---
tags: [core, cw-illustrate]
---

# cw-illustrate

> The pictures of a CodeWiki note: their components, their limits and the file that keeps them.

## What it does

`cw-illustrate` is the one place that defines what a picture in a CodeWiki note may be. A coding agent writes a picture as a short OpenUI Lang program with four components: Picture, Flow, Step and Callout. The app draws it natively; nothing is fetched or run. This crate holds the component list, the size limits, the check a submission must pass, and the file format that stores a note's pictures. Because the backend check and the app renderer share it, they cannot drift apart.

## Source

- `crates/cw-illustrate/Cargo.toml` — package manifest
- `crates/cw-illustrate/src/lib.rs` — crate root

## How it works

`library` declares the four components and their properties. `check` parses a source and evaluates it against that library. `review` is the gate used on submission. It returns one line per fault, and an empty list means the picture may be kept. It tests, in turn:

1. Size of the source, no backticks, no double square brackets, and no line starting with the section marker.
2. Parse and component errors from the parser.
3. The root must be a Picture, holding only Flow and Callout, with at most one Flow and two Callouts.
4. A Flow has 2 to 7 Steps, and every title, text and tone is within its limit.

`limits` prints the same numbers in words; the brief given to writing agents uses it. `quote` turns text into a safe string literal. `parse_pictures` and `serialize_pictures` read and write the per-note file, described in [[Picture]].

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

## Gotchas

- The limits are constants, and the words in `limits` and the checks in `review` both come from them. Change a constant and the briefs change at once.
- The checks count characters for text and bytes for the whole source.
- Backticks and double square brackets are refused because the app shows text as written, so a link would appear with its brackets.
- Old components such as Card, Stat, BarChart and Table are gone and fail as unknown components.
- Callouts accept an empty tone, which means info.

## See also

- [[_index]]
- [[Crate Dependency Flow]]

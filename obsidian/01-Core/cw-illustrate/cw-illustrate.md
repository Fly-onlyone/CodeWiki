---
tags: [core, cw-illustrate]
---

# cw-illustrate

> CodeWiki's illustration component library and structural illustration builders.

## What it does

CodeWiki's illustration component library and structural builders.

An illustration is an OpenUI Lang program saved beside a note as `<Note>.openui`. The app draws it natively; nothing is fetched or executed. This crate is the single definition of which components exist, so the generator, the validator and the renderer cannot drift apart.

## Source

- `crates/cw-illustrate/Cargo.toml` — package manifest
- `crates/cw-illustrate/src/lib.rs` — crate root

## How it works

Builds a library.

1. `generate_with` in [[Generate Module]] calls `check`. `check` calls `evaluate` in [[Eval Module]], then `parse` in [[Parser Module]], then `library`. `library` calls `Library::new` in [[Library]].
2. `RenderCtx::illustration` in [[RenderCtx]] calls `overview`. `overview` calls `quote`, then `figure_row`, then `number`, then `card`. `figure_row` calls `quote`. `card` calls `quote`.
3. `RenderCtx::illustration` in [[RenderCtx]] calls `steps`. `steps` calls `quote`, then `step_list`, then `card`. `step_list` calls `quote`.
4. `RenderCtx::illustration` in [[RenderCtx]] calls `brief`. `brief` calls `figure_row`, then `step_list`, then `card`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Defined at the crate root:

- `pub const EXTENSION: &str` — file extension of saved illustrations.
- `pub fn library() -> Library` — the components an illustration may use. Layout: `Card`, `Stack`, `Grid`, `Tabs`/`Tab`, `Divider`. Content: `Header`, `TextContent`, `Stat`, `Badge`, `Steps`/`Step`, `Table`/`Col`. Charts: `BarChart`, `LineChart`, `PieChart` with `Series`.
- `pub fn check(source: &str) -> Document` — parse and check an illustration against the library.
- `pub fn quote(text: &str) -> String` — quote text as an OpenUI Lang string literal.
- `fn number(value: f64) -> String` — private function, returns `String`; called by `overview`.
- [[Figure]] — one headline figure.
- `fn card(body: &[&str], title: &str) -> String` — the frame of an illustration: a card around the parts named in `body`. An empty `title` gives a card without one, for an illustration that stands under its note's own title.
- `fn figure_row(figures: &[Figure]) -> String` — the assignments of a row of headline figures, named `figures`.
- `fn step_list(items: &[(String, String)]) -> String` — the assignments of a numbered sequence, named `flow`.
- `overview` — an overview card: headline figures and an optional bar chart, under an optional title and one-line summary. Built from structural facts only, so it needs no LLM. Leave `title` and `summary` empty for an illustration shown under its note's own.
- `pub fn steps(title: &str, summary: &str, items: &[(String, String)]) -> String` — A numbered sequence of steps, for flows, under an optional title and one-line summary.
- `pub fn brief(figures: &[Figure], steps: &[(String, String)]) -> Option<String>` — what a note is about at a glance: headline figures, then the steps of its walk-through. It has no title and no summary, since it stands under the note's own. `None` when there is neither a figure nor a step.

## Depends on

- [[Eval Module]] — uses `Document`, `evaluate`
- [[Library]] — uses `Library`, `new`
- [[Parser Module]] — uses `parse`
- [[Figure]] — uses `Figure`

## Used by

- [[cw-wiki]] — from `crate_and_flow_notes_get_valid_illustrations`, `module_type_and_integration_notes_get_a_picture_of_facts`
- [[Generate Module]] — from `generate_with`, `illustration_path`
- [[codewiki-app]] — from `open_note`
- [[RenderCtx]] — from `illustration`

## See also

- [[_index]]
- [[Crate Dependency Flow]]

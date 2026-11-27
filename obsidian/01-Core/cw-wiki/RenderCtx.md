---
tags: [core, cw-wiki]
---

# RenderCtx

> `RenderCtx` struct defined in `cw_wiki::render` with 21 methods.

## What it does

Holds everything needed to write the rule-made part of every planned note: the repository lookup tables ([[Repo]]), the [[Plan Module]] result, the note registry and the repository name. It also precomputes a one-line summary for every note, and which notes depend on which. The generator builds one `RenderCtx` per run and calls `render` for each planned note.

## Source

- `crates/cw-wiki/src/render.rs` — definition of `RenderCtx`

## How it works

`new` walks all dependency edges. When both ends have an owner note and the owners differ, it records the link in two maps: `depends_on` (from owner to owner, with symbol names) and `used_by` (the reverse). It then calls `summary` for every planned note and stores the results, so `render` can print a summary without work.

`link` turns a note id into a wikilink. It returns `None` unless the note is planned in this run (or adopted). The registry may still hold notes from older runs that are about to be deleted, and linking to those would leave dead links. Map notes get a link with a label from `zone_label`.

`render` builds a `Doc` and calls the renderer for the note's subject. Crate, module and type notes share one shape: What it does from the doc comment, Source, How it works from the explain walk, Items, Depends on, Used by, See also. Flow notes carry a Mermaid diagram: a flowchart for crate dependencies, a sequence diagram for startup messages. Integration notes list who imports the package and what build files ask of it. Pattern notes show a code skeleton in the language of the trait. The operations note lists commands read from build files, with `unit_commands` covering Python, TypeScript, JavaScript and Java. `render_index` and `render_home` write the maps and the reading path by hand.

## Items

Declared as `pub struct RenderCtx<'a>`.

Members:

- `new` — public method, returns `Self`; called by `generate_with`.
- `pub fn link(&self, id: &str) -> Option<String>` — wikilink to the note with `id`, or `None` when it is not in the vault.
- `fn name(&self, id: &str) -> String` — private method, returns `String`; called by `RenderCtx::new`, `RenderCtx::render`, `RenderCtx::render_crate`, `RenderCtx::render_home` and 4 more.
- `fn summary(&self, note: &Planned) -> String` — private method, returns `String`; called by `RenderCtx::new`, `RenderCtx::render_crate`, `RenderCtx::render_home`.
- `fn unit_language(&self, unit: &Unit) -> String` — the language a unit is called after: the one with more files. A Gradle or Maven module that holds both Java and Kotlin is named for both, the larger first (`Kotlin and Java`, and Kotlin on a tie).
- `fn composition(&self) -> (Vec<String>, Vec<String>)` — the languages of the repository and how many units each has, the language with the most source files first.
- `fn entry_label(&self, entry: &str) -> String` — how a reader names the entry point `entry`: `main` in Rust, the file for a script or for the top-level statements of a C# file, `Class.main` in Java.
- `fn dep_bullets(&self, map: &NoteDeps, id: &str, verb: &str) -> String` — bullets for a note-level dependency map.
- `pub fn render(&self, note: &Planned) -> String` — public method, returns `String`.
- `fn unit_commands(&self, unit: &Unit) -> Vec<String>` — commands that install, run and test a unit written in another language than Rust, read off its build files and entry points.

And 11 more private items with no comment.

## Depends on

- [[Render Module]] — uses `Doc`, `HandZone`, `MAX_HAND_NOTES`, `MAX_LISTED`
- [[Filemap Module]] — uses `folder`, `folder_sections`, `folder_summary`, `home_summary`
- [[Plan Module]] — uses `FlowMessage`, `Plan`, `Planned`, `ZONE_ARCHITECTURE`
- [[Repo]] — uses `Repo`, `children_of`, `module_count`, `subtree_rank`
- [[Doctext Module]] — uses `clean_inline`, `doc_body`, `ensure_period`, `first_sentence`
- [[Units]] — uses `all_rust`, `by_name`, `c_tests`, `of_file`
- [[Explain Module]] — uses `items`, `text`, `walk`
- [[Pm Module]] — uses `install`, `run_script`

## Used by

- [[Explain Module]] — from `callers_sentence`, `facts`, `items`, `text`
- [[Generate Module]] — from `generate_with`

## Gotchas

- Summaries are computed in `new`, before any note is rendered. A summary that needs a link would see only the plan, not rendered text.
- `dep_bullets` shows at most 8 notes and 4 symbol names each. Linked notes that are not planned are skipped without a trace.
- Startup flow notes warn that calls through trait objects, macros or untyped values may be missing. The diagram only follows resolved calls.
- The operations note assumes standard Cargo commands. Other languages get commands only when a manifest was read.

## See also

- [[_index]]
- [[Render Module]]

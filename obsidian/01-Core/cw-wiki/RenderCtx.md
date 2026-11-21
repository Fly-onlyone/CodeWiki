---
tags: [core, cw-wiki]
---

# RenderCtx

> `RenderCtx` struct defined in `cw_wiki::render` with 20 methods.

## Source

- `crates/cw-wiki/src/render.rs` — definition of `RenderCtx`

## How it works

1. `generate_with` in [[Generate Module]] calls `new`. `new` calls `name`, then `summary`. `summary` calls `first_sentence`, then `ensure_period`, then `clean_inline` (all in [[Doctext Module]]), then `language_name`, then `unit_noun`, then `count` (all in [[Render Module]]), and 5 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct RenderCtx<'a>`.

Members:

- `new` — public method, returns `Self`; called by `generate_with`.
- `pub fn link(&self, id: &str) -> Option<String>` — wikilink to the note with `id`, or `None` when it is not in the vault.
- `fn name(&self, id: &str) -> String` — private method, returns `String`; called by `RenderCtx::new`, `RenderCtx::render`, `RenderCtx::render_crate`, `RenderCtx::render_startup_flow` and 2 more.
- `fn summary(&self, note: &Planned) -> String` — private method, returns `String`; called by `RenderCtx::new`, `RenderCtx::render_crate`, `RenderCtx::render_home`.
- `fn composition(&self) -> (Vec<String>, Vec<String>)` — the languages of the repository and how many units each has, the language with the most source files first.
- `fn entry_label(&self, entry: &str) -> String` — how a reader names the entry point `entry`: `main` in Rust, the file for a script, `Class.main` in Java.
- `fn dep_bullets(&self, map: &NoteDeps, id: &str, verb: &str) -> String` — bullets for a note-level dependency map.
- `pub fn render(&self, note: &Planned) -> String` — public method, returns `String`.
- `fn unit_commands(&self, unit: &Unit) -> Vec<String>` — commands that install, run and test a unit written in another language than Rust, read off its build files and entry points.

And 11 more private items with no comment.

## Depends on

- [[Render Module]] — uses `Doc`, `MAX_LISTED`, `MAX_MANIFESTS`, `MAX_NAMES`
- [[Plan Module]] — uses `FlowMessage`, `Plan`, `Planned`, `ZONE_ARCHITECTURE`
- [[Repo]] — uses `Repo`, `children_of`, `module_count`, `subtree_rank`
- [[Doctext Module]] — uses `clean_inline`, `doc_body`, `ensure_period`, `first_sentence`
- [[Explain Module]] — uses `items`, `text`, `walk`
- [[Language]] — uses `Language`, `from_path`
- [[Registry]] — uses `Registry`, `by_id`
- [[Unit]] — uses `Unit`, `is_rust`

## Used by

- [[Explain Module]] — from `callers_sentence`, `facts`, `items`, `text`
- [[Generate Module]] — from `generate_with`

## See also

- [[_index]]
- [[Render Module]]

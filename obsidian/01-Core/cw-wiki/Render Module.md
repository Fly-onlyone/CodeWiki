---
tags: [core, cw-wiki, module]
---

# Render Module

> Rendering of planned notes into Markdown in the vault's house style.

## What it does

Turns each planned note into Markdown, with no model involved. Every sentence comes from structural facts (symbols, edges, manifests) and from doc comments in the code. The result is the factual skeleton of a note: source, items, dependencies. Agents later add the explanation sections on top. The note layout produced here is the contract the vault linter enforces. The module holds the small text helpers; the main work is in [[RenderCtx]].

## Source

- `crates/cw-wiki/src/render.rs` — module `cw_wiki::render`

## How it works

`RenderCtx::render` makes a `Doc` from the note's front matter, title and one-line summary. It then picks a renderer by the note's subject: crate, module, type, crate flow, startup flow, integration, pattern or operations. Each renderer adds named sections to the `Doc`. A section with an empty body is dropped. `Doc::finish` joins them through the vault's own note layout, so agent-written sections and rule-written ones share one format. Map notes (`_index`) and the home note are built by hand and skip `Doc`.

The helpers are small. `join_and` writes lists with an Oxford comma. `count` pluralises. `zone_label` turns `01-Core` into `Core`. `language_name`, `unit_noun` and `external_noun` choose words such as crate or package. `build_role` names a build file. `lower_first` lowercases the first letter of a summary so it fits after a dash in a bullet.

## Items

- `const MAX_MANIFESTS: usize` — build files named on one line of an integration note.
- `const MAX_UNIT_MODULES: usize` — modules listed in the note of a unit that has no root module.
- `const MAX_HAND_NOTES: usize` — notes the home note names of a folder of hand-written notes that has no map note of its own.
- `type NoteDeps = HashMap<String, BTreeMap<String, BTreeSet<String>>>` — Note-level dependency edges: note id -> other note id -> symbol names.
- `pub struct HandZone` — A top folder of the vault that holds notes their owner wrote by hand.
- [[RenderCtx]] — struct
- `struct Doc` — accumulates a note's sections; empty ones are dropped.
- `fn front(tags: &[String]) -> String` — private function, returns `String`; called by `Doc::new`, `RenderCtx::render_home`, `RenderCtx::render_index`.
- `fn zone_label(segment: &str) -> &str` — `01-Core` -> `Core`; other folder names are kept.
- `fn top_children<'a>(repo: &Repo<'a>, parent: &str, limit: usize) -> Vec<&'a Symbol>` — private function, returns `Vec<&'a Symbol>`; called by `RenderCtx::summary`.
- `fn join_and(items: &[String]) -> String` — private function, returns `String`; called by `RenderCtx::render_crate`, `RenderCtx::render_crate_flow`, `RenderCtx::render_operations`, `RenderCtx::render_pattern` and 2 more.
- `fn count(n: usize, noun: &str) -> String` — `1 module`, `3 modules`.
- `fn language_name(language: Language) -> &'static str` — private function, returns `&'static str`; called by `RenderCtx::composition`, `RenderCtx::summary`.
- `fn unit_noun(language: Language) -> &'static str` — what one unit of a language is called.
- `fn external_noun(files: &BTreeSet<&str>) -> &'static str` — `crate` when only Rust files import the dependency, else `package`.
- `fn build_role(path: &str) -> &'static str` — what a build file outside Cargo is for.
- `pub(crate) fn lower_first(text: &str) -> String` — lowercase the first letter unless the text starts with code or an acronym.
- `const PROPER_NOUNS: [&str; 17]` — product and technology names that keep their capital at the start of a list item.

And 2 more private items with no comment.

## Depends on

- [[Repo]] — uses `Repo`, `children_of`, `subtree_rank`
- [[Language]] — uses `Language`, `from_path`
- [[Note Module]] — uses `join`
- [[Plan Module]] — uses `Planned`
- [[Symbol]] — uses `Symbol`
- [[Section]] — uses `Section`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `composition`, `dep_bullets`, `link`
- [[Explain Module]] — from `items`
- [[Generate Module]] — from `generate_with`

## Gotchas

- `lower_first` keeps the capital for code, acronyms, CamelCase words, one-letter words and the names in `PROPER_NOUNS`. A new product name must be added to that list, or it will be lowered mid-list.
- `zone_label` only strips a two-digit prefix. `cw-core` stays as it is.
- `Doc::section` keeps empty sections until `finish`, which drops them. Do not rely on a heading being there.

## See also

- [[_index]]
- [[cw-wiki]]

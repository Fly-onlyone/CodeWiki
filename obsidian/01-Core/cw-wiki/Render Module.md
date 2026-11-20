---
tags: [core, cw-wiki, module]
---

# Render Module

> Rendering of planned notes into Markdown in the vault's house style.

## What it does

This is the keyless path: every sentence is derived from structural facts and doc comments. An LLM writer may later replace the prose, but the structure produced here is the contract the linter enforces.

## Source

- `crates/cw-wiki/src/render.rs` — module `cw_wiki::render`

## How it works

1. `items` in [[Explain Module]] and `RenderCtx::render_crate` in [[RenderCtx]], and 2 more, call `lower_first`.
2. `RenderCtx::render` in [[RenderCtx]] calls `Doc::new`. `Doc::new` calls `front`.
3. `RenderCtx::render_home` and `RenderCtx::render_index` in [[RenderCtx]] call `front`.
4. `RenderCtx::link` in [[RenderCtx]] calls `zone_label`.
5. `RenderCtx::summary` in [[RenderCtx]] calls `top_children`.

6 more entry points: `join_and`, `count`, `language_name`, `unit_noun`, `external_noun`, `build_role`.

## Items

- `const MAX_MANIFESTS: usize` — build files named on one line of an integration note.
- `const MAX_UNIT_MODULES: usize` — modules listed in the note of a unit that has no root module.
- `type NoteDeps = HashMap<String, BTreeMap<String, BTreeSet<String>>>` — Note-level dependency edges: note id -> other note id -> symbol names.
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
- [[Symbol]] — uses `Symbol`
- [[Planned]] — uses `Planned`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `composition`, `dep_bullets`, `illustration`
- [[Explain Module]] — from `items`

## See also

- [[_index]]
- [[cw-wiki]]

---
tags: [core, cw-wiki, module]
---

# Plan Module

> Concept selection: decide which notes a repository gets.

## What it does

The vault holds curated concept notes, not a page per file. Selection uses only structural facts, so it works without an LLM and is deterministic.

## Source

- `crates/cw-wiki/src/plan.rs` — module `cw_wiki::plan`

## How it works

1. `Repo::is_test` in [[Repo]] calls `is_test_path`.
2. `generate_with` in [[Generate Module]] calls `plan`. `plan` calls `module_dir`, then `module_title`, then `enclosing_module`, then `startup_messages`, then `title_case`. `module_title` calls `title_case`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const ZONE_ARCHITECTURE: &str` — public constant; used by `RenderCtx::render_home`, `RenderCtx::summary`, `plan`.
- `pub const ZONE_CORE: &str` — public constant; used by `RenderCtx::render_home`, `RenderCtx::summary`, `module_dir`, `plan`.
- `pub const ZONE_INTEGRATIONS: &str` — public constant; used by `RenderCtx::render_home`, `RenderCtx::summary`, `plan`.
- `pub const ZONE_PATTERNS: &str` — public constant; used by `RenderCtx::render_home`, `RenderCtx::summary`, `plan`.
- `pub const ZONE_OPERATIONS: &str` — public constant; used by `RenderCtx::render_home`, `RenderCtx::summary`, `plan`.
- `const MAX_FLOWS_PER_UNIT: usize` — startup flows drawn for one unit with several entry points.
- `const MIN_MODULE_ITEMS: usize` — outside Rust every file is a module, so only those that hold this many items, or this many symbols counting nested ones, get a note.
- `pub struct FlowMessage` — one step of a startup flow: `from` note calls into `to` note.
- `pub enum Subject` — public enum; used by `Planned`.
- [[Planned]] — struct
- `pub struct Plan` — public struct; used by `RenderCtx`, `RenderCtx::new`, `generate_with`, `plan`.
- [[Repo]] — lookup tables over the scanned facts.
- `fn module_title(module: &Symbol) -> String` — title of the note about `module`. A TypeScript `index` file is named after its folder.
- `pub fn title_case(snake: &str) -> String` — `hash` -> `Hash`, `note_lifecycle` -> `Note Lifecycle`.
- `fn is_test_path(qualified: &str) -> bool` — private function, returns `bool`; called by `Repo::is_test`.
- `fn module_dir(unit: &Unit, module: &str) -> String` — vault folder for notes about `module` and the types it defines.
- `fn enclosing_module<'a>(repo: &Repo<'a>, symbol: &'a Symbol) -> &'a str` — nearest enclosing module of a symbol (itself when it is a module).
- `pub fn plan(repo: &Repo, covered: &HashMap<String, String>) -> Plan` — select notes. `covered` maps source files already documented by adopted notes to the id of the adopted note; those files get no new module or type notes.
- `fn startup_messages(repo: &Repo, owner: &HashMap<String, String>, entry: &str) -> Vec<FlowMessage>` — follow calls outward from an entry point and keep those that cross from one note into another.

And 7 more private items with no comment.

## Depends on

- [[Repo]] — uses `Repo`, `calls_from`, `children_of`, `descendants`
- [[Unit]] — uses `Unit`, `is_rust`
- [[Units]] — uses `all_rust`, `index_of`
- [[Language]] — uses `has_symbol_support`
- [[Symbol]] — uses `Symbol`
- [[SymbolKind]] — uses `is_type`
- [[Planned]] — uses `Planned`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `new`, `render_home`, `render_startup_flow`
- [[Generate Module]] — from `generate_with`
- [[Planned]] — from `Planned`
- [[Repo]] — from `is_test`

## See also

- [[_index]]
- [[cw-wiki]]

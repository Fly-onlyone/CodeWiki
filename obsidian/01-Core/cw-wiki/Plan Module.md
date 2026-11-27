---
tags: [core, cw-wiki, module]
---

# Plan Module

> Concept selection: decide which notes a repository gets.

## What it does

Decides which wiki notes a repository gets. The vault is a set of curated concept notes, not one page per file, so something must pick the crates, modules, types, flows, integrations and patterns worth a note. `plan` does this from structural facts only (symbols, edges, ranks, manifests). It uses no model, so the same facts always give the same plan. The note generator calls it, and the renderer reads the resulting `Plan`.

## Source

- `crates/cw-wiki/src/plan.rs` — module `cw_wiki::plan`

## How it works

`Repo::new` or `Repo::at` builds lookup tables over the scanned facts: symbols by name, children by parent, callers and callees, trait implementors, and page rank. `Repo::at` also reads the manifests from disk.

`plan` then adds notes in this order:

1. One crate note per unit (a crate, or its counterpart in another language).
2. Module notes for modules that define something. In Rust, a private inline module needs 3 or more items. Outside Rust every file is a module, so it needs 3 items or 5 nested symbols. Tests and files already covered by an adopted note are skipped. The best 300 by rank are kept.
3. Type notes: per unit, the public types in the top quarter by rank, plus any with 3 or more children or 2 or more implementors, at most 12.
4. Ownership: each symbol is mapped to the nearest enclosing note, falling back to its crate note.
5. A crate dependency flow when there are 2 or more units, and up to 3 startup flows per unit.
6. Integrations: external crates used in enough files (2 for small repos, 3 otherwise), at most 15.
7. Patterns: traits with 2 or more implementors, at most 10.
8. A build note, then one `_index` map per folder and ancestor folder, then the home note.

`startup_messages` walks calls breadth-first from an entry point, to depth 4. It keeps only calls that cross from one note's owner to another, and stops at 14 messages. A flow with fewer than 2 messages is dropped.

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
- `pub struct Planned` — public struct; used by `Doc::new`, `Plan`, `RenderCtx::render`, `RenderCtx::render_crate` and 13 more.
- `pub struct Plan` — public struct; used by `RenderCtx`, `RenderCtx::new`, `generate_with`, `plan`.
- [[Repo]] — lookup tables over the scanned facts.
- `fn module_title(module: &Symbol) -> String` — title of the note about `module`. A TypeScript `index` file is named after its folder.
- `pub fn title_case(snake: &str) -> String` — `hash` -> `Hash`, `note_lifecycle` -> `Note Lifecycle`.
- `fn is_test_path(qualified: &str) -> bool` — private function, returns `bool`; called by `Repo::is_test`.
- `fn module_dir(unit: &Unit, module: &str) -> String` — vault folder for notes about `module` and the types it defines.
- `fn enclosing_module<'a>(repo: &Repo<'a>, symbol: &'a Symbol) -> &'a str` — nearest enclosing module of a symbol (itself when it is a module).
- `fn cut<T>(ranked: &mut Vec<T>, limit: usize, pinned: impl Fn(&T) -> bool)` — keep the first `limit` of `ranked`, and behind them every one that is pinned: a limit drops the least important notes, never an explained one.
- `pub fn plan(repo: &Repo, covered: &HashMap<String, String>, pinned: &HashSet<String>) -> Plan` — select notes. `covered` maps source files already documented by adopted notes to the id of the adopted note; those files get no new module or type notes. `pinned` holds the ids of the notes a coding agent explained. Which modules, types, integrations and patterns get a note is a ranking with limits, and a ranking moves when other code changes. A pinned note keeps its place for as long as what it describes exists, so that an explanation is lost only with its subject.
- `fn startup_messages(repo: &Repo, owner: &HashMap<String, String>, entry: &str) -> Vec<FlowMessage>` — follow calls outward from an entry point and keep those that cross from one note into another.

And 7 more private items with no comment.

## Depends on

- [[Repo]] — uses `Repo`, `calls_from`, `children_of`, `descendants`
- [[Unit]] — uses `Unit`, `is_rust`
- [[Units]] — uses `all_rust`, `index_of`
- [[Language]] — uses `has_symbol_support`
- [[Symbol]] — uses `Symbol`
- [[SymbolKind]] — uses `is_type`
- [[NoteKind]] — uses `NoteKind`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `new`, `render`, `render_crate`
- [[Generate Module]] — from `carry_moves`, `generate_with`, `planned_path`
- [[Render Module]] — from `new`
- [[Repo]] — from `is_test`

## Gotchas

- Ownership is computed before flows. `startup_messages` needs `owner`, so a symbol with no owner is invisible to a flow.
- Files in `covered` get no module or type notes, and their symbols belong to the adopted note.
- Each selection is sorted by rank, cut to a cap, then sorted again by name. Caps such as 300 modules and 12 types per crate silently drop the rest.
- `is_test_path` matches whole path segments (`tests`, `test`, `benches`, `examples`), so `testing_tools` is not a test.
- A unit's first declared entry carries the unit's name. Later entries are named after themselves.

## See also

- [[_index]]
- [[cw-wiki]]

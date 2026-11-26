---
tags: [core, cw-wiki]
---

# Repo

> Lookup tables over the scanned facts.

## What it does

A read-only index over one scan of the repository. The scan gives flat lists of symbols and edges. `Repo` turns them into maps so planning and rendering can ask quick questions: who are the children of a symbol, which calls does it make, which unit holds it, how important is it, which types implement a trait. It also holds the repository's units (crates or packages) and what its build files say about outside packages.

## Source

- `crates/cw-wiki/src/plan.rs` — definition of `Repo`

## How it works

`Repo::new` builds the tables from facts alone. `Repo::at` also reads manifests from the checked-out folder, so it can name non-Rust units and read package descriptions. Both go through `build`, which loops once over symbols (by name, children by parent) and once over edges (outgoing, incoming, and implementors for `Implements` edges). Rank comes from the page rank in the metrics.

Queries are plain lookups. `calls_from` keeps only call edges, orders them by call site and keeps each callee once. `unit_of` finds the unit by symbol, or by Rust path when the symbol is unknown. `top_modules` lists the modules right below a unit's root; for units with no root it finds modules not nested in another module of the unit. `descendants` and `subtree_rank` add up everything nested below a symbol. `is_test` is true for test paths and for test files outside Rust.

## Items

Declared as `pub struct Repo<'a>`.

Members:

- `pub fn new(facts: &'a RepoFacts, metrics: &'a [(String, NodeMetrics)]) -> Self` — lookup tables from facts alone. Units outside Rust are named from the layout only; use `Repo::at` to also read their manifests.
- `pub fn at(root: &Path, facts: &'a RepoFacts, metrics: &'a [(String, NodeMetrics)]) -> Self` — lookup tables for the repository checked out at `root`.
- `build` — private method, returns `Self`; called by `Repo::at`, `Repo::new`.
- `pub fn calls_from(&self, qualified: &str) -> Vec<&'a Edge>` — the calls `qualified` makes, in the order they stand in its source, each callee once.
- `pub fn unit_index(&self, qualified: &str) -> Option<usize>` — index of the unit that holds the symbol named `qualified`.
- `pub fn unit_of(&self, qualified: &str) -> Option<&Unit>` — the unit that holds the symbol named `qualified`.
- `pub fn top_modules(&self, unit: &Unit) -> Vec<&'a Symbol>` — modules directly below the top of `unit`, in name order.
- `pub fn module_count(&self, unit: &Unit) -> usize` — number of source modules in `unit`. A package's `__init__` file is the package itself, not one of its modules.
- `pub fn descendants(&self, qualified: &str) -> usize` — number of symbols nested anywhere below `qualified`.
- `pub fn is_test(&self, symbol: &Symbol) -> bool` — true for test code, which gets no notes of its own.
- `pub fn rank_of(&self, qualified: &str) -> f64` — public method, returns `f64`; called by `Repo::subtree_rank`.
- `pub fn children_of(&self, qualified: &str) -> &[&'a Symbol]` — public method, returns `&[&'a Symbol]`; called by `Repo::descendants`, `Repo::subtree_rank`, `Repo::top_modules`.
- `pub fn subtree_rank(&self, qualified: &str) -> f64` — rank of a symbol plus everything nested in it.

## Depends on

- [[Units]] — uses `Units`, `index_of`, `rust_index`
- [[Manifests Module]] — uses `Packages`, `read`
- [[Units Module]] — uses `discover`, `is_test_file`
- [[Unit]] — uses `Unit`, `is_rust`
- [[Model Module]] — uses `Edge`
- [[Store Module]] — uses `NodeMetrics`
- [[Scan Module]] — uses `RepoFacts`
- [[Plan Module]] — uses `is_test_path`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `new`, `render_crate`, `render_home`
- [[Explain Module]] — from `facts`, `items`, `walk`
- [[Plan Module]] — from `enclosing_module`, `plan`, `startup_messages`
- [[Manifests Module]] — from `declared`, `describe`
- [[Generate Module]] — from `generate_with`
- [[Render Module]] — from `top_children`

## Gotchas

- `Repo::new` names non-Rust units from layout only. Use `Repo::at` when manifests matter.
- `rank_of` returns 0 for an unknown symbol, not an error.
- `module_count` skips `__init__` files, because a Python package's `__init__` is the package, not one of its modules.
- `children` is built in fact order, not sorted by name, despite the field doc. Callers that need an order sort themselves.

## See also

- [[_index]]
- [[Plan Module]]

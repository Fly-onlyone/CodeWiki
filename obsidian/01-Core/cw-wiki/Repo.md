---
tags: [core, cw-wiki]
---

# Repo

> Lookup tables over the scanned facts.

## Source

- `crates/cw-wiki/src/plan.rs` — definition of `Repo`

## How it works

1. `generate_with` in [[Generate Module]] calls `at`. `at` calls `build`, then `discover` in [[Units Module]], then `Packages::read` in [[Manifests Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

- [[RenderCtx]] — from `RenderCtx`, `illustration`, `new`, `render_crate`
- [[Explain Module]] — from `facts`, `items`, `walk`
- [[Plan Module]] — from `enclosing_module`, `plan`, `startup_messages`
- [[Manifests Module]] — from `declared`, `describe`
- [[Generate Module]] — from `generate_with`
- [[Render Module]] — from `top_children`

## See also

- [[_index]]
- [[Plan Module]]

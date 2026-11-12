---
tags: [core, cw-wiki]
---

# Repo

> Lookup tables over the scanned facts.

## Source

- `crates/cw-wiki/src/plan.rs` — definition of `Repo`

## How it works

Declared as `pub struct Repo<'a>`.

Public API:

- `pub fn unit_of(&self, qualified: &str) -> Option<&Unit>` — the unit that holds the symbol named `qualified`.
- `pub fn unit_index(&self, qualified: &str) -> Option<usize>` — index of the unit that holds the symbol named `qualified`.
- `pub fn subtree_rank(&self, qualified: &str) -> f64` — rank of a symbol plus everything nested in it.
- `pub fn top_modules(&self, unit: &Unit) -> Vec<&'a Symbol>` — modules directly below the top of `unit`, in name order.
- `pub fn at(root: &Path, facts: &'a RepoFacts, metrics: &'a [(String, NodeMetrics)]) -> Self` — lookup tables for the repository checked out at `root`.
- `pub fn module_count(&self, unit: &Unit) -> usize` — number of source modules in `unit`.

## Depends on

- [[Units]] — uses `Units`, `index_of`, `rust_index`
- [[Units Module]] — uses `discover`, `is_test_file`
- [[Unit]] — uses `Unit`, `is_rust`
- [[Model Module]] — uses `Edge`
- [[Store Module]] — uses `NodeMetrics`
- [[Scan Module]] — uses `RepoFacts`
- [[Plan Module]] — uses `is_test_path`
- [[Language]] — uses `from_path`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `illustration`, `new`, `render_crate`
- [[Plan Module]] — from `enclosing_module`, `plan`, `startup_messages`
- [[Generate Module]] — from `generate`
- [[Render Module]] — from `top_children`

## See also

- [[_index]]
- [[Plan Module]]

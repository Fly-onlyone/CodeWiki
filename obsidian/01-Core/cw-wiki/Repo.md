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

Queries are plain lookups. `calls_from` keeps only call edges, orders them by call site and keeps each callee once. `unit_of` finds the unit by symbol, or by Rust path when the symbol is unknown. `top_modules` lists the modules right below a unit's root; for units with no root it scans all modules of the unit and drops those nested in another module of the unit. `descendants` and `subtree_rank` add up everything nested below a symbol.

`is_test` decides what gets no note. A path with a test segment counts in any language. For languages other than Rust, a test file, a file in a test unit, or a symbol marked as generated code also counts. `top_modules` and `module_count` use it to skip Go packages that hold only tests.

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
- `fn is_go_test_package(&self, module: &Symbol) -> bool` — true for a Go package that holds tests alone (an external test package, or a folder of `_test.go` files): no part of the code.
- `pub fn is_test(&self, symbol: &Symbol) -> bool` — true for test code, which gets no notes of its own. Generated Go code is kept out the same way: its symbols exist and are called, but a note about code nobody writes would rank among those that matter.
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
- [[Go Module]] — uses `GENERATED_ATTR`
- [[Scan Module]] — uses `RepoFacts`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `new`, `render_crate`, `render_home`
- [[Plan Module]] — from `enclosing_module`, `plan`, `startup_messages`
- [[Explain Module]] — from `items`, `walk`
- [[Manifests Module]] — from `declared`, `describe`
- [[Generate Module]] — from `generate_with`
- [[Render Module]] — from `top_children`

## Gotchas

- `Repo::new` names non-Rust units from layout only. Use `Repo::at` when manifests matter.
- `rank_of` returns 0 for an unknown symbol, not an error.
- `module_count` skips `__init__` files, because a Python package's `__init__` is the package, not one of its modules.
- `children` is built in fact order. The field doc says name order, but the code never sorts. Callers that need an order sort themselves.
- Generated Go code counts as test code in `is_test`. It stays in the index and keeps its calls, but gets no note. Generated Rust is not filtered this way.

## See also

- [[_index]]
- [[Plan Module]]

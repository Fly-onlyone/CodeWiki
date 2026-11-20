---
tags: [core, cw-wiki, module]
---

# Health Module

> Code health: dead code, documentation drift and per-file health scores.

## What it does

Every detector here is written from first principles against CodeWiki's own index. Findings state how sure they are, because name resolution runs without a compiler.

## Source

- `crates/cw-wiki/src/health.rs` — module `cw_wiki::health`

## How it works

1. `run` in [[codewiki]] and `file_health` in [[Api Module]], and 2 more, call `file_health`. `file_health` calls `dead_code`, then `parameter_count`.
2. `run` in [[codewiki]] and `dead_code` in [[Api Module]], and 1 more, call `dead_code`. `dead_code` calls `is_test_symbol`, then `Language::from_path` in [[Language]].
3. `run` in [[codewiki]] and `doc_drift` in [[Api Module]], and 1 more, call `doc_drift`. `doc_drift` calls `VaultMeta::load` in [[VaultMeta]], then `state` in [[Lifecycle Module]], then `Note::parse` in [[Note]], then `looks_like_path`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum Certainty` — public enum; used by `DeadSymbol`.
- `pub struct DeadSymbol` — public struct; used by `dead_code`.
- `pub(crate) const PASSIVE_ATTRS: [&str; 12]` — attributes that only describe an item and never make it reachable.
- `fn is_test_symbol(symbol: &Symbol) -> bool` — private function, returns `bool`; called by `dead_code`.
- `pub fn dead_code(index: &Index) -> Result<Vec<DeadSymbol>>` — symbols nothing refers to. Reported: functions, methods, types, constants and statics with no incoming call, reference or implementation edge. Skipped: entry points, tests, trait items and trait-impl methods (reached by dynamic dispatch), enum and struct members, and anything public in a library crate (it is API).
- `pub enum DriftKind` — public enum; used by `Drift`.
- `pub struct Drift` — public struct; used by `DocReport`, `doc_drift`.
- `pub struct DocReport` — public struct; used by `doc_drift`.
- `fn looks_like_path(span: &str) -> bool` — does a backticked span look like a repo path (`src/a.rs`, `crates/x/`)?
- `pub fn doc_drift(root: &Path, index: &Index) -> Result<DocReport>` — where the documentation no longer matches the code.
- `pub struct Finding` — public struct; used by `FileHealth`, `file_health`.
- `pub struct FileHealth` — public struct; used by `file_health`.
- `pub(crate) const MANY_METHODS: usize` — internal constant; used by `extract_types`, `file_health`.
- `fn parameter_count(signature: &str) -> usize` — number of parameters in a function signature, not counting a receiver. Counts top-level commas of the first parenthesised group, so generic arguments and nested tuples do not inflate it.
- `pub fn file_health(index: &Index) -> Result<Vec<FileHealth>>` — health score for every source file with symbol support, worst first.

And 7 more private items with no comment.

## Depends on

- [[Index]] — uses `Index`, `crates`, `edges`, `files`
- [[Language]] — uses `from_path`, `has_symbol_support`
- [[VaultMeta]] — uses `load`, `registry`
- [[cw-core]] — uses `VAULT_DIR`
- [[Error Module]] — uses `Result`
- [[Model Module]] — uses `FileRecord`
- [[Lifecycle Module]] — uses `state`
- [[Analyze Module]] — uses `DEPENDENCY_KINDS`

## Used by

- [[cw-wiki]] — from `a_cycle_is_two_sibling_files_not_a_module_and_its_parent`, `finds_dead_code_without_false_alarms`, `private_items_used_through_receivers_and_constants_are_alive`, `reports_documentation_drift_and_coverage`
- [[Api Module]] — from `dead_code`, `doc_drift`, `file_health`
- [[Refactor Module]] — from `extract_types`, `move_functions`, `split_files`
- [[codewiki]] — from `run`
- [[Ci Module]] — from `check`

## See also

- [[_index]]
- [[cw-wiki]]

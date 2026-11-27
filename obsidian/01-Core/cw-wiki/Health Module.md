---
tags: [core, cw-wiki, module]
---

# Health Module

> Code health: dead code, documentation drift and per-file health scores.

## What it does

This module finds three kinds of trouble: code nothing uses, notes that no longer match the code, and files that are too big or too tangled. It reads CodeWiki's own index, not a compiler, so names are matched without full type information. Each dead-code finding therefore says how sure it is. The results feed the app, the API and the CI check.

## Source

- `crates/cw-wiki/src/health.rs` — module `cw_wiki::health`

## How it works

`dead_code` marks a symbol as referenced when it has an incoming call, reference or implement edge. A type is alive when any of its methods is used. It skips modules, impls, macros, tests, `main`, public methods of traits or of types that implement a trait, and anything public in a library crate (that is API). What remains is `Likely` only when the symbol is Rust, private and has no attribute that could register it. Otherwise it is `Possible`, with the reason in words: an attribute like a route macro may wire it up, or the language is dynamic. Likely findings come first, then longer ones.

`doc_drift` goes through every note. It reports a cited source file that is gone, a source that changed since a hand-edited note was reviewed, a wikilink that matches no note, and a backticked path that is not in the repository. It also reports documentation coverage: the share of source files that some note cites, and the 20 largest uncited files.

`file_health` starts each source file at 10 and subtracts penalties. Detectors: long file (600 or 1200 lines), long function (80 or 160), large type (25 methods), high fan-out (15 files), mutual dependency, many parameters (7), undocumented API, wide API (30 public items), and dead code. The lowest score is 1. Files are sorted worst first. `parameter_count` ignores the receiver and nested commas.

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
- [[Note Module]] — uses `parse`

## Used by

- [[cw-wiki]] — from `a_cycle_is_two_sibling_files_not_a_module_and_its_parent`, `a_note_that_waits_to_be_retired_is_no_note_of_the_wiki`, `finds_dead_code_without_false_alarms`, `private_items_used_through_receivers_and_constants_are_alive`
- [[Api Module]] — from `dead_code`, `doc_drift`, `file_health`
- [[Refactor Module]] — from `extract_types`, `move_functions`, `split_files`
- [[codewiki]] — from `run`
- [[Ci Module]] — from `check`

## Gotchas

- Only Rust symbols can be `Likely` dead. Other languages are `Possible`, because reflection and decorators are invisible to the index.
- A module and the file that declares it with `mod` do not count as a mutual dependency.
- Generated notes that nobody edited are not checked for dead paths. They copy source comments, where paths are often examples, and the next sync rewrites them.
- A source change counts as drift only for hand-edited notes. Generated notes are rewritten on sync.
- A backticked path is only checked when its first folder exists in the repository or vault, which avoids flagging paths of other projects.

## See also

- [[_index]]
- [[cw-wiki]]

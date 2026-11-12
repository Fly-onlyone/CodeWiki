---
tags: [core, cw-wiki, module]
---

# Health Module

> Code health: dead code, documentation drift and per-file health scores.

## Source

- `crates/cw-wiki/src/health.rs` — module `cw_wiki::health`

## How it works

Main items:

- `DriftKind` — enum
- `Drift` — struct
- `Finding` — struct
- `Certainty` — enum
- `dead_code` — symbols nothing refers to.
- `file_health` — health score for every source file with symbol support, worst first.
- `doc_drift` — where the documentation no longer matches the code.
- `DeadSymbol` — struct

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

- [[cw-wiki]] — from `finds_dead_code_without_false_alarms`, `reports_documentation_drift_and_coverage`, `scores_file_health_with_reasons`
- [[Api Module]] — from `dead_code`, `doc_drift`, `file_health`
- [[Refactor Module]] — from `extract_types`, `move_functions`, `split_files`
- [[codewiki]] — from `run`
- [[Ci Module]] — from `check`

## See also

- [[_index]]
- [[cw-wiki]]
